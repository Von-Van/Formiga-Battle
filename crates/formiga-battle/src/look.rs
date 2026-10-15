//! How each creature looks: the very frames Desktop draws for it, from the look Desktop sent, never
//! a lookalike.

use formiga_art::{
    AccessoryArt, AnimationSpec, BodyClip, Canvas, CreatureRenderer, ExpressionKind, EyelidPose,
    FRAME_SIZE, FaceRenderState, GazeDirection,
};
use formiga_battle_contract::Contender;
use formiga_core::{ActionKind, AppearanceGenome, Gesture};
use std::collections::HashMap;

/// What a creature's body is doing on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pose {
    Idle,
    Walk,
    /// Standing its ground: settled and watching.
    Hold,
    /// Reaching for whoever it is contesting.
    Reach,
    /// Caught on the move.
    Gasp,
    /// Pressed while holding, and wobbling.
    Wobble,
    /// Bumped, or tangled in a scuffle.
    Worry,
    /// Knocked out: flopped over, dizzy.
    Dizzy,
    Cheer,
}

impl Pose {
    fn clip(self) -> BodyClip {
        match self {
            Self::Idle => BodyClip::Action(ActionKind::Idle),
            Self::Walk => BodyClip::Action(ActionKind::Traverse),
            Self::Hold => BodyClip::Gesture(Gesture::Watch),
            Self::Reach => BodyClip::Gesture(Gesture::Reach),
            Self::Gasp => BodyClip::Gesture(Gesture::Gasp),
            Self::Wobble => BodyClip::Gesture(Gesture::Balance),
            Self::Worry => BodyClip::Gesture(Gesture::Worry),
            Self::Dizzy => BodyClip::Action(ActionKind::Sleep),
            Self::Cheer => BodyClip::Gesture(Gesture::Cheer),
        }
    }

    fn face(self) -> (ExpressionKind, EyelidPose) {
        match self {
            Self::Idle => (ExpressionKind::Content, EyelidPose::Open),
            Self::Walk => (ExpressionKind::Focused, EyelidPose::Open),
            Self::Hold => (ExpressionKind::Determined, EyelidPose::Open),
            Self::Reach => (ExpressionKind::Determined, EyelidPose::Open),
            Self::Gasp => (ExpressionKind::Startled, EyelidPose::Open),
            Self::Wobble => (ExpressionKind::Worried, EyelidPose::Open),
            Self::Worry => (ExpressionKind::Worried, EyelidPose::Open),
            Self::Dizzy => (ExpressionKind::Sleepy, EyelidPose::Closed),
            Self::Cheer => (ExpressionKind::Joy, EyelidPose::Open),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct FrameKey {
    pose: Pose,
    frame: u8,
    facing_right: bool,
}

/// The most frames kept for one creature before the cache starts again.
const FRAME_CACHE_LIMIT: usize = 96;

pub struct Look {
    pub name: String,
    genome: AppearanceGenome,
    dress: Option<AccessoryArt>,
    /// The frame row its feet rest on.
    pub foot_row: i32,
    reduce_motion: bool,
    frames: HashMap<FrameKey, Canvas>,
}

impl Look {
    pub fn of(contender: &Contender, reduce_motion: bool) -> anyhow::Result<Self> {
        let creature = contender.traveler.to_creature()?;
        let baseline = CreatureRenderer::resting_baseline(&creature.appearance, reduce_motion);
        Ok(Self {
            name: contender.traveler.name.clone(),
            genome: creature.appearance,
            dress: contender.traveler.accessory.map(|a| a.to_art()),
            foot_row: FRAME_SIZE as i32 - 1 - baseline as i32,
            reduce_motion,
            frames: HashMap::new(),
        })
    }

    /// Its frame for a pose, `elapsed` seconds into it. Reduced motion holds each pose's first
    /// frame.
    pub fn frame(&mut self, pose: Pose, elapsed: f32, facing_right: bool) -> &Canvas {
        let clip = pose.clip();
        let frame = if self.reduce_motion {
            0
        } else {
            AnimationSpec::for_clip(clip).frame_at(elapsed)
        };
        let key = FrameKey {
            pose,
            frame,
            facing_right,
        };
        if self.frames.len() >= FRAME_CACHE_LIMIT && !self.frames.contains_key(&key) {
            self.frames.clear();
        }
        let (genome, dress, still) = (&self.genome, self.dress, self.reduce_motion);
        self.frames.entry(key).or_insert_with(|| {
            let (expression, eyelids) = pose.face();
            CreatureRenderer::render_dressed_composited_frame(
                genome,
                dress,
                clip,
                frame,
                facing_right,
                still,
                FaceRenderState {
                    expression,
                    eyelids,
                    gaze: GazeDirection::default(),
                },
            )
        })
    }

    /// A still portrait for its card.
    pub fn portrait(&mut self) -> Canvas {
        self.frame(Pose::Idle, 0.0, true).clone()
    }
}
