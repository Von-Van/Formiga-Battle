//! The friend's half of a sitting: a seat in a room on the relay, through
//! `formiga_battle_room`. Opening or joining the room, leaving it, what the relay says each frame,
//! seating the match once both teams are in, and working out each locked turn here and telling
//! the relay the board it came to. The rest of the sitting (orders, locking, the turn shown) is
//! the same for a friend as for the visitors, and stays with the table.

use super::*;

impl Table {
    /// Opens a room on the relay for a friend (`code` is `None`), or joins the friend's room by
    /// its code.
    pub fn open_room(&mut self, handler: &str, code: Option<&str>) {
        self.notice = None;
        if self.picked.len() != TEAM_SIZE {
            self.notice = Some("Choose three first.".to_owned());
            return;
        }
        if self.checking_board() {
            self.notice = Some(STILL_CHECKING.to_owned());
            return;
        }
        let (team, bonds) = self.picked_team();
        // The name signed here is the Handler's from now on.
        if let Some(card) = crate::identity::card_with_name(&self.record, handler)
            && self.record.handler.as_ref() != Some(&card)
        {
            self.set_handler(card);
        }
        let entrant = Entrant {
            protocol: formiga_battle_room::RELAY_VERSION,
            rules_version: formiga_battle_core::RULES_VERSION,
            build: self.build(),
            handler: handler.trim().to_owned(),
            team,
            bonds,
            arena: self.arena.id.clone(),
            card: self.shared_card(),
            handler_key: self.handler_key(),
        };
        self.friend = Some(Friend {
            line: Line::start(self.relay.clone(), entrant, code.map(str::to_owned)),
            code: None,
            seated: false,
            handlers: [String::new(), String::new()],
            cards: [None, None],
            view: None,
            sent_turn: None,
            opponent_ready: false,
            opponent_away: false,
            deadline: 0.0,
            held: None,
            gone: false,
        });
    }

    /// The colony's anonymous Handler key, as Desktop keeps it, for a friend's Battle to know this
    /// Handler again. A rehearsal has none, whatever roster it was given.
    pub fn handler_key(&self) -> Option<String> {
        self.session.as_ref()?;
        self.record
            .handler_key
            .clone()
            .filter(|key| formiga_battle_contract::is_handler_key(key))
    }

    /// What this Battle plays with: the Desktop that opened it, or none in a rehearsal. The line
    /// to the relay adds the program's digest.
    fn build(&self) -> Build {
        Build::this(self.session.as_ref().map(|_| self.desktop_version.clone()))
    }

    /// Leaves the room, or the match with a friend. Leaving a match forfeits it.
    pub fn leave_room(&mut self) {
        if let Some(friend) = self.friend.take() {
            friend.line.ask(Ask::Leave);
        }
    }

    /// Whether what the last match with a friend earned is still waiting on the relay to say
    /// both boards came out the same. Opening another room or taking on the visitors would
    /// drop it, so both wait.
    pub fn checking_board(&self) -> bool {
        self.friend.as_ref().is_some_and(|f| f.held.is_some())
    }

    /// Leaves the room once a match with a friend is over and what it earned is settled, so the
    /// next room can be opened without waiting for the relay to close this one. A room whose
    /// boards are still being checked is kept, and the earnings with it.
    pub fn leave_when_settled(&mut self) {
        if self.outcome().is_some()
            && self.friend.as_ref().is_some_and(|f| f.seated)
            && !self.checking_board()
        {
            self.leave_room();
        }
    }

    /// Whatever the relay has said since the last frame. True if a match with a friend began.
    pub fn poll(&mut self, now: f32) -> bool {
        let mut began = false;
        while let Some(told) = self.friend.as_ref().and_then(|f| f.line.told()) {
            match told {
                Told::Seated { code } => {
                    if let Some(f) = &mut self.friend {
                        f.code = Some(code);
                    }
                }
                Told::View(view) => {
                    self.notice = None;
                    began |= self.seen(*view, now);
                }
                Told::Problem(why) => self.notice = Some(why),
                Told::Gone(why) => {
                    if self.outcome().is_none() {
                        self.notice = Some(why);
                    }
                    self.settle_held(
                        false,
                        "The room closed before both boards could be checked, so this match \
                         earned nothing.",
                    );
                    // A match the room closed under stays on the board, ended here, until the
                    // Handler leaves it: no more turns can come, and it earns nothing.
                    let playing = self.friend.as_ref().is_some_and(|f| f.seated);
                    if !playing || self.outcome().is_some() {
                        self.friend = None;
                    } else if let Some(friend) = &mut self.friend {
                        friend.gone = true;
                    }
                    break;
                }
            }
        }
        self.catch_up(now);
        // Orders not sent by the time the turn locks are sent at the last moment.
        if let Some(friend) = &self.friend
            && friend.seated
            && !friend.gone
            && self.shown.is_none()
            && self.outcome().is_none()
            && self
                .m
                .as_ref()
                .is_some_and(|m| !friend.sent(m.board.turn + 1))
            && friend.deadline - now < 2.0
        {
            self.lock(now);
        }
        began
    }

    /// A new view of the room.
    fn seen(&mut self, view: View, now: f32) -> bool {
        let mine = self.build();
        let Some(friend) = &mut self.friend else {
            return false;
        };
        friend.deadline = now + view.deadline_ms as f32 / 1000.0;
        friend.opponent_ready = view.opponent_ready;
        friend.opponent_away = view.opponent_quiet_ms > 8_000;
        if view.out_of_step {
            self.notice = Some(
                "Your board and your friend's came out differently; this match cannot be trusted."
                    .to_owned(),
            );
        }
        if friend.held.is_some() && (view.confirmed || view.out_of_step) {
            let confirmed = view.confirmed;
            self.settle_held(
                confirmed,
                "Your board and your friend's came out differently, so this match earned nothing.",
            );
        }
        let Some(friend) = &mut self.friend else {
            return false;
        };
        let mut began = false;
        if !friend.seated
            && let Some([a, b]) = &view.entrants
        {
            // The relay checks nothing of the rules: each window checks both teams itself, and that
            // both sides play the same Battle and Desktop by the rules this Battle knows.
            let unfit = [a, b].into_iter().find_map(|e| {
                if e.rules_version != formiga_battle_core::RULES_VERSION
                    || e.build.battle != mine.battle
                    || e.build.desktop != mine.desktop
                    || view.ruleset != Ruleset::default()
                {
                    Some("Your Battle and your friend's are different versions; both need the newest.".to_owned())
                } else {
                    check_team(&e.team, &e.bonds)
                        .err()
                        .map(|reason| format!("A team cannot play: {reason}."))
                }
            });
            if let Some(why) = unfit {
                self.notice = Some(why);
                self.leave_room();
                return false;
            }
            // The opener's arena, which both Battles must have: they are the same version.
            let Some(arena) = Arena::by_id(&a.arena) else {
                self.notice = Some(
                    "Your friend picked an arena this Battle does not have; both need the newest."
                        .to_owned(),
                );
                self.leave_room();
                return false;
            };
            let handlers = [handler_name(&a.handler), handler_name(&b.handler)];
            match room_match(
                arena,
                view.ruleset,
                view.seed,
                [&a.team, &b.team],
                [&a.bonds, &b.bonds],
            ) {
                Ok(m) => {
                    friend.seated = true;
                    friend.handlers = handlers.clone();
                    friend.cards = [
                        a.card.as_ref().and_then(Card::cleaned),
                        b.card.as_ref().and_then(Card::cleaned),
                    ];
                    let teams = [a.team.clone(), b.team.clone()];
                    let you = view.you;
                    friend.view = Some(view);
                    self.seat_friend_match(m, you, &handlers, teams, now);
                    return true;
                }
                Err(error) => self.notice = Some(format!("The match could not begin: {error}")),
            }
        }
        if let Some(friend) = &mut self.friend {
            friend.view = Some(view);
            began = false;
        }
        began
    }

    /// Works out the next locked turn not yet shown here, once the last has been shown, and
    /// tells the relay the board it came to. A forfeit the relay reports ends the match.
    fn catch_up(&mut self, now: f32) {
        if self.shown.is_some() {
            return;
        }
        let Some(friend) = &self.friend else {
            return;
        };
        let Some(view) = &friend.view else {
            return;
        };
        let Some(m) = &mut self.m else {
            return;
        };
        if !friend.seated {
            return;
        }
        let next = view
            .locked
            .iter()
            .find(|l| l.turn == m.board.turn + 1)
            .cloned();
        let ending = view.ending.clone();
        if let Some(locked) = &next
            && m.outcome().is_none()
        {
            let before = m.board.clone();
            let report = m.resolve_turn(locked.submitted.clone());
            friend.line.ask(Ask::Board {
                turn: report.turn,
                hash: report.hash,
                finished: m.outcome().is_some(),
            });
            self.show_turn(before, report, now);
            return;
        }
        if let Some(ending) = ending
            && m.outcome().is_none()
            && next.is_none()
            && let Some(side) = ending.forfeited
        {
            m.forfeit(side);
            self.ended_because = Some(formiga_expansion_rulebook::sanitize_text(
                &ending.reason,
                REASON_CHARS,
            ));
            self.finish_match();
        }
    }

    /// Puts a match with a friend on the board, from either side.
    fn seat_friend_match(
        &mut self,
        mut m: Match,
        you: Side,
        handlers: &[String; 2],
        teams: [Vec<Contender>; 2],
        now: f32,
    ) {
        // A friend's companion who shares a name with one of the Handler's is told apart.
        let ours: Vec<String> = m.rosters[you.index()]
            .iter()
            .map(|p| p.name.clone())
            .collect();
        for p in &mut m.rosters[you.other().index()] {
            if ours.contains(&p.name) {
                p.name = format!("{}'s {}", handlers[you.other().index()], p.name);
            }
        }
        let looks: anyhow::Result<Vec<Vec<Look>>> = teams
            .iter()
            .map(|team| {
                team.iter()
                    .map(|c| Look::of(c, self.reduce_motion))
                    .collect()
            })
            .collect();
        match looks {
            Ok(mut looks) => {
                for (look, p) in looks[you.other().index()]
                    .iter_mut()
                    .zip(&m.rosters[you.other().index()])
                {
                    look.name = p.name.clone();
                }
                let right = looks.pop().expect("two teams");
                let left = looks.pop().expect("two teams");
                self.looks = [left, right];
            }
            Err(error) => {
                self.notice = Some(format!("A companion's look could not be drawn: {error}"));
                return;
            }
        }
        self.you = you;
        self.other_name = format!("{}'s team", handlers[you.other().index()]);
        self.other_handler = Some(handlers[you.other().index()].clone());
        self.board_teams = teams;
        self.begin_match(m, now);
    }
}
