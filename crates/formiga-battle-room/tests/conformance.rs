//! The relay conformance cases, run against the reference relay. The Formiga site keeps a
//! byte-identical copy of `conformance/relay-cases.json` and runs the same cases against its
//! relay, so the two relays cannot drift apart unnoticed.

use formiga_battle_core::TeamPlan;
use formiga_battle_room::relay::{Answer, Limits, Refusal, Request};
use formiga_battle_room::{Entrant, Relay};
use serde_json::Value;
use std::collections::BTreeMap;

const CASES: &str = include_str!("conformance/relay-cases.json");

fn counting() -> Box<dyn FnMut() -> u64 + Send> {
    let mut n = 11u64;
    Box::new(move || {
        n = n
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        n
    })
}

/// What a body is read as, as the relay's HTTP layer reads it: anything that is not one is
/// malformed before the relay sees it.
fn read<T: serde::de::DeserializeOwned>(body: &Value) -> Option<T> {
    serde_json::from_value(body.clone()).ok()
}

/// The value at a dotted path: a number picks from a list, `length` counts one, and anything
/// missing is null.
fn at(value: &Value, path: &str) -> Value {
    let mut here = value;
    for part in path.split('.') {
        match (here, part) {
            (Value::Array(items), "length") => return Value::from(items.len()),
            (Value::Array(items), index) => {
                match index.parse::<usize>().ok().and_then(|i| items.get(i)) {
                    Some(next) => here = next,
                    None => return Value::Null,
                }
            }
            (Value::Object(fields), key) => match fields.get(key) {
                Some(next) => here = next,
                None => return Value::Null,
            },
            _ => return Value::Null,
        }
    }
    here.clone()
}

#[test]
fn the_reference_relay_answers_every_conformance_case_as_written() {
    let doc: Value = serde_json::from_str(CASES).expect("the cases are JSON");
    let entrants = &doc["entrants"];
    let cases = doc["cases"].as_array().expect("a list of cases");
    assert!(!cases.is_empty());
    for case in cases {
        let name = case["name"].as_str().expect("a name");
        let mut relay = Relay::with_entropy(Limits::default(), counting());
        let mut code = String::new();
        let mut seats: BTreeMap<String, String> = BTreeMap::new();
        for (n, step) in case["steps"].as_array().expect("steps").iter().enumerate() {
            let now = step["at"].as_u64().expect("a time");
            let side = step["as"].as_str().expect("a side").to_owned();
            let seat = seats.get(&side).cloned().unwrap_or_default();
            let entrant = |v: &Value| -> Option<Entrant> { read(v) };
            let request: Option<Request> = if let Some(who) = step["open"].as_str() {
                entrant(&entrants[who]).map(|entrant| Request::Open { entrant })
            } else if let Some(who) = step["join"].as_str() {
                entrant(&entrants[who]).map(|entrant| Request::Join {
                    code: code.clone(),
                    entrant,
                })
            } else if step["join_body"].is_object() {
                entrant(&step["join_body"]).map(|entrant| Request::Join {
                    code: code.clone(),
                    entrant,
                })
            } else if step["look"].as_bool() == Some(true) {
                Some(Request::Look {
                    code: code.clone(),
                    seat,
                })
            } else if step["leave"].as_bool() == Some(true) {
                Some(Request::Leave {
                    code: code.clone(),
                    seat,
                })
            } else if step["orders"].is_object() {
                let body = &step["orders"];
                let turn = body["turn"].as_u64().and_then(|t| u8::try_from(t).ok());
                let orders: Option<TeamPlan> = read(&body["orders"]);
                turn.zip(orders).map(|(turn, orders)| Request::Orders {
                    code: code.clone(),
                    seat,
                    turn,
                    orders,
                })
            } else if step["board"].is_object() {
                let body = &step["board"];
                let turn = body["turn"].as_u64().and_then(|t| u8::try_from(t).ok());
                let hash = body["hash"]
                    .as_str()
                    .filter(|h| h.len() == 16)
                    .and_then(|h| u64::from_str_radix(h, 16).ok());
                let finished = body["finished"].as_bool();
                match (turn, hash, finished) {
                    (Some(turn), Some(hash), Some(finished)) => Some(Request::Board {
                        code: code.clone(),
                        seat,
                        turn,
                        hash,
                        finished,
                    }),
                    _ => None,
                }
            } else {
                panic!("{name}, step {n}: no request");
            };
            let answer = match request {
                Some(request) => relay.handle(request, now),
                None => Answer::Refused(Refusal::Malformed),
            };
            if let Answer::Seat { code: c, seat: s } = &answer {
                code.clone_from(c);
                seats.insert(side.clone(), s.clone());
            }
            let status = answer.status();
            let body = serde_json::to_value(&answer).expect("an answer is JSON");
            let expect = &step["expect"];
            assert_eq!(
                Some(u64::from(status)),
                expect["status"].as_u64(),
                "{name}, step {n}: status, answered {body}"
            );
            if let Some(wanted) = expect.get("body") {
                assert_eq!(&body, wanted, "{name}, step {n}: body");
            }
            if let Some(Value::Object(fields)) = expect.get("view") {
                for (path, wanted) in fields {
                    assert_eq!(
                        &at(&body, path),
                        wanted,
                        "{name}, step {n}: {path} in {body}"
                    );
                }
            }
        }
    }
}
