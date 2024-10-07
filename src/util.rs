use macroquad::math::{vec2, Vec2};

pub fn pythogoras(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

#[derive(Clone)]
pub struct LineSegment {
    pub start: Vec2,
    pub end: Vec2,
}

pub struct LineSegmentPointsOn {
    interval: f32,
    current: Option<Vec2>,
    end: Vec2,
}

impl Iterator for LineSegmentPointsOn {
    type Item = Vec2;

    fn next(&mut self) -> Option<Vec2> {
        let current = self.current?;
        let mut next = Some(vec2(
            current.x
                + self.interval
                    * ((self.end.x - current.x)
                        / pythogoras(self.end.x - current.x, self.end.y - current.y)),
            current.y
                + self.interval
                    * ((self.end.y - current.y)
                        / pythogoras(self.end.x - current.x, self.end.y - current.y)),
        ));
        match pythogoras(self.end.x - current.x, self.end.y - current.y)
            - pythogoras(next.unwrap().x - current.x, next.unwrap().y - current.y)
        {
            n if n > 0.0 => {
                // self.current = Some(next.clone());
                std::mem::swap(&mut self.current, &mut next);
                next
            }
            _ => {
                self.current = None;
                None
            }
        }
    }
}

//https://stackoverflow.com/a/2049593
fn sign(p1: Vec2, p2: Vec2, p3: Vec2) -> f32 {
    return (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y);
}
fn in_triange(p: Vec2, v1: Vec2, v2: Vec2, v3: Vec2) -> bool {
    let d1 = sign(p, v1, v2);
    let d2 = sign(p, v2, v3);
    let d3 = sign(p, v3, v1);

    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);

    return !(has_neg && has_pos);
}

impl LineSegment {
    pub fn new<A, B>(start: A, end: B) -> Self
    where
        A: Into<Vec2>,
        B: Into<Vec2>,
    {
        LineSegment {
            start: start.into(),
            end: end.into(),
        }
    }

    pub fn points_on(&self, amount: usize) -> LineSegmentPointsOn {
        let interval =
            pythogoras(self.start.x - self.end.x, self.start.y - self.end.y) / amount as f32;
        LineSegmentPointsOn {
            interval,
            current: Some(self.start.clone()),
            end: self.end.clone(),
        }
    }

    pub fn lies_between(&self, other: &Self, point: Vec2) -> bool {
        in_triange(point, self.start, self.end, other.start)
            || in_triange(point, other.start, other.end, self.start)
            || in_triange(point, other.start, other.end, self.end)
    }
}
