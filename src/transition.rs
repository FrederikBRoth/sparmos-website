use std::collections::BTreeMap;

use cgmath::Point3;
use sparmos_engine::cgmath;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VoxelObjects {
    Home,
    CSharp,
    CPlusPLus,
    Rust,
    Containerization,
    Castle,
    Viking,
    Buttplug,
    HandballBird,
    FemogfirsSlangen,
    BadApple,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]

pub enum CameraPositions {
    Middle((Point3<i32>, Point3<i32>)),
    LeftSide((Point3<i32>, Point3<i32>)),
    RightSide((Point3<i32>, Point3<i32>)),
    FrontAndCenter((Point3<i32>, Point3<i32>)),
}

pub struct TransitionHandler<T> {
    pub transition_map: BTreeMap<i64, T>,
    pub last_transition: Option<T>,
    pub last_position: i64,
}

impl<T: Clone + PartialEq> TransitionHandler<T> {
    pub fn new(transition_map: BTreeMap<i64, T>) -> Self {
        Self {
            transition_map,
            last_transition: None,
            last_position: 0,
        }
    }

    pub fn get_transition(&mut self, position: i64) -> Option<T> {
        let mut start = 0;
        let mut transition: Option<T> = None;
        for (&n, value) in self.transition_map.iter() {
            if is_between(start, n, position) {
                transition = Some(value.clone());
                break;
            }
            start = n;
        }

        self.last_transition = transition.clone();
        transition
    }

    pub fn get_transition_once(&mut self, position: i64) -> Option<T> {
        let mut start = 0;
        let mut transition: Option<T> = None;
        for (&n, value) in self.transition_map.iter() {
            if is_between(start, n, position) {
                transition = Some(value.clone());
                break;
            }
            start = n;
        }

        if let Some(last_trans) = &self.last_transition
            && let Some(trans) = &transition
            && last_trans == trans
        {
            return None;
        }
        self.last_transition = transition.clone();
        transition
    }

    pub fn get_transition_once_exact(&mut self, position: i64) -> Option<T> {
        let transition: Option<T> = self.transition_map.get(&position).cloned();

        if let Some(last_trans) = &self.last_transition
            && let Some(trans) = &transition
            && last_trans == trans
        {
            return None;
        }
        self.last_transition = transition.clone();
        transition
    }

    pub fn get_transition_per_movement(&mut self, position: i64) -> (i64, i64, Option<T>) {
        let mut start = 0;
        let mut transition: Option<T> = None;

        let (mut end, mut normalized_position) = (0, 0);
        for (&n, value) in self.transition_map.iter() {
            if is_between(start, n, position) {
                end = n - start;
                normalized_position = position - start;
                transition = Some(value.clone());
                break;
            }
            start = n;
        }

        if self.last_position == normalized_position || normalized_position == 0 {
            transition = None
        }
        self.last_position = normalized_position;

        (end, normalized_position, transition)
    }
}

fn is_between(start: i64, end: i64, number: i64) -> bool {
    number >= start && end > number
}
