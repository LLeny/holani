use super::{alloc, Deserialize, Serialize};
use alloc::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepKind {
    BusFrontier,
    InternalFrontier,
    StretchRow,
    TiltRow,
    CollisionHalfPair,
    VideoMerge,
    CollisionBus,
    CollisionGroup,
    ClipFifo,
    XorByte,
    ScbFetch,
    SpriteEndFetch,
    ScbStartup,
    ControlLine,
    VerticalSkip,
    MathMultiply,
    MathDivide,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepQueue {
    steps: VecDeque<StepKind>,
}

impl StepQueue {
    pub fn push(&mut self, kind: StepKind, n: u16) {
        for _ in 0..n {
            self.steps.push_back(kind);
        }
    }

    pub fn pop_front(&mut self) -> Option<StepKind> {
        self.steps.pop_front()
    }

    pub fn pop_back_n(&mut self, n: u32) {
        for _ in 0..n {
            self.steps.pop_back();
        }
    }

    pub fn drop_front_n(&mut self, n: usize) {
        for _ in 0..n {
            self.steps.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.steps.clear();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn append(&mut self, other: &mut Self) {
        self.steps.append(&mut other.steps);
    }
}

