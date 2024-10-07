use std::{
    collections::HashMap,
    fmt::{self, Display},
};

use macroquad::{
    color::*,
    input::{is_mouse_button_down, is_mouse_button_pressed, mouse_position, MouseButton},
    math::{vec2, Vec2},
    rand::gen_range,
    shapes::draw_rectangle,
    text::draw_text,
    texture::{draw_texture, load_texture, FilterMode, Texture2D},
    window::{clear_background, next_frame, Conf},
};

mod util;
use macroquad_particles::{AtlasConfig, BlendMode, EmissionShape, Emitter, EmitterConfig};
pub use util::*;

const GAME_WIDTH: u16 = 960;
const GAME_HEIGHT: u16 = 540;
const STOE_SHIFT: f32 = 0.3;
const MAX_DIRTINESS: u8 = 200;
const WARN_DIRTINESS: u8 = 180;
#[rustfmt::skip]
const NOISE: [f32; 100] = [0.0, 0.37567067, 0.9067937, 0.47849727, 0.53902316, 0.68121976, 0.8017116, 0.3828842, 0.09980044, 0.28901517, 0.819964, 0.07882048, 0.9314874, 0.2782374, 0.8892265, 0.7379155, 0.8957271, 0.28707007, 0.38089857, 0.65332454, 0.012101332, 0.6167583, 0.821882, 0.05945961, 0.92279524, 0.03035006, 0.7336123, 0.98893404, 0.99925655, 0.35572338, 0.9292264, 0.88346875, 0.85185605, 0.68569475, 0.14773135, 0.6225942, 0.48433545, 0.1802073, 0.17406808, 0.26091358, 0.25314412, 0.3917573, 0.21147245, 0.88591653, 0.06278534, 0.45477942, 0.21266633, 0.92625904, 0.5458369, 0.9122172, 0.5397183, 0.035206992, 0.428736, 0.40691206, 0.754005, 0.49157023, 0.384951, 0.520259, 0.692683, 0.3089388, 0.65079826, 0.29621452, 0.8601855, 0.5781134, 0.63684237, 0.9962076, 0.3542669, 0.8180771, 0.7678995, 0.82436645, 0.72423524, 0.2671644, 0.56586105, 0.77570736, 0.11471661, 0.6794964, 0.8524261, 0.1201895, 0.21402203, 0.9767727, 0.5880526, 0.4113872, 0.8640513, 0.026697583, 0.12278987, 0.36087683, 0.86676, 0.082543656, 0.76316553, 0.6951772, 0.28111908, 0.70043737, 0.43776283, 0.086626664, 0.05120758, 0.5787454, 0.01473637, 0.8254751, 0.46910095, 0.42112306];

fn conf() -> Conf {
    Conf {
        window_title: "Sokoban".into(),
        window_width: GAME_WIDTH as i32,
        window_height: GAME_HEIGHT as i32,
        fullscreen: false,
        window_resizable: false,
        ..Default::default()
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum ResourceName {
    ImageA1,
    ImageB1,
    Ant,
    AntSV,
    AntSVCrit,
    AntSH,
    AntSHCrit,
    AntCrit,
}
type Resources = HashMap<ResourceName, Texture2D>;

struct GameState {
    screen: Screen,
    levels: HashMap<Stage, LevelState>,
}

enum Screen {
    MainMenu,
    Stage(Stage, LevelState),
    DeathAnim(Stage, u8),
    Dialog(Dialog),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Stage {
    A1,
    B1,
}
impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Stage::A1 => "A1",
            Stage::B1 => "B2",
        })
    }
}

enum Dialog {
    Lost(Stage),
    Won(Stage, Stage),
}

#[derive(Clone)]
struct DirtyObj {
    dirtiness: u8,
    start: LineSegment,
    end: LineSegment,
    amount: usize,
    distance: Vec2,
    chance_bidir: u8,
}

#[derive(Clone, PartialEq, Eq)]
enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Clone)]
struct LevelState {
    objects: Vec<DirtyObj>,
    difficulty: Difficulty,
    scene: ResourceName,
    money: f32,
    money_goal: f32,
    repellants: u16,
    repellant_name: String,
    repellant_price: f32,
}

fn manage_level(
    level_state: &mut LevelState,
    resources: &Resources,
    tick: usize,
    duration: &mut Option<usize>,
) -> Option<Screen> {
    static MEDIUM_DURATION: usize = 5 * 60 * 60;
    use ResourceName::*;
    //draw scene
    draw_texture(resources.get(&level_state.scene).unwrap(), 0.0, 0.0, WHITE);

    match level_state.difficulty {
        Difficulty::Easy => {}
        Difficulty::Medium => match duration {
            Some(d) => {
                *d += 1;
            }
            None => {
                *duration = Some(0);
            }
        },
        Difficulty::Hard => todo!(),
    }

    let mut objects_complete_dirty = 0;
    let mut num_objects = 0;
    let mouse_pos = mouse_position();

    //draw ants
    for (object_id, object) in &mut level_state.objects.iter_mut().enumerate() {
        // let dirtiness = f32::floor(object.dirtiness as f32 / 20.0) * 20.0;
        // let speed = (f32::powi(dirtiness, 3)) * 0.007;

        for (idx, (start, end)) in object
            .start
            .points_on(object.amount)
            .zip(object.end.points_on(object.amount))
            .enumerate()
        {
            let (ant, (off_x, off_y)) = match level_state.difficulty {
                Difficulty::Easy => match object.dirtiness > WARN_DIRTINESS {
                    false => (resources.get(&Ant).unwrap(), (4.0, 4.0)),
                    true => (
                        resources
                            .get(if NOISE[idx % 100] > 0.9 {
                                &AntCrit
                            } else {
                                &Ant
                            })
                            .unwrap(),
                        (4.0, 4.0),
                    ),
                },
                Difficulty::Medium => match object_id {
                    1 => match object.dirtiness > WARN_DIRTINESS {
                        false => (resources.get(&AntSH).unwrap(), (1.0, 0.0)),
                        true => (
                            resources
                                .get(if NOISE[idx % 100] > 0.7 {
                                    &AntSHCrit
                                } else {
                                    &AntSH
                                })
                                .unwrap(),
                            (1.0, 0.0),
                        ),
                    },
                    _ => match object.dirtiness > WARN_DIRTINESS {
                        false => (resources.get(&AntSV).unwrap(), (0.0, 1.0)),
                        true => (
                            resources
                                .get(if NOISE[idx % 100] > 0.7 {
                                    &AntSVCrit
                                } else {
                                    &AntSV
                                })
                                .unwrap(),
                            (0.0, 1.0),
                        ),
                    },
                },
                Difficulty::Hard => todo!(),
            };

            let (x, y) = match lerp_ant(tick, &object, start, end, idx, true) {
                Some(p) => p,
                None => {
                    continue;
                }
            };

            draw_texture(
                ant,
                (x - off_x) + (gen_range(-1.0, 1.0)) * (0.01 * object.distance.y),
                (y - off_y) + (gen_range(-1.0, 1.0)) * (0.01 * object.distance.x),
                WHITE,
            );
        }

        let update_dirtiness: bool = match level_state.difficulty {
            Difficulty::Easy => {
                let rep_effect = (3.0 * (level_state.repellants as f32 / 10.0)) as usize;
                tick % (7 + rep_effect) == 0
            }
            Difficulty::Medium => {
                // let rep_effect = (2.0 + (5.0 / level_state.repellants as f32)) as usize;
                tick % 7 == 0
            }
            Difficulty::Hard => {
                let rep_effect = (level_state.repellants / 10) as usize;
                tick % (2 + rep_effect) == 0
            }
        };

        if update_dirtiness {
            object.dirtiness = match level_state.difficulty {
                Difficulty::Easy => object.dirtiness.checked_add(1).unwrap_or(object.dirtiness),
                Difficulty::Medium => object.dirtiness.checked_sub(1).unwrap_or(object.dirtiness),
                Difficulty::Hard => todo!(),
            };
        }

        // kill ants or make money
        if is_mouse_button_down(MouseButton::Left) {
            if object.start.lies_between(&object.end, mouse_pos.into()) {
                // println!(
                //     "Clicked in object {object_id}, dirtiness now: {}",
                //     object.dirtiness
                // );
                match level_state.difficulty {
                    Difficulty::Easy => {
                        object.dirtiness =
                            object.dirtiness.checked_sub(1).unwrap_or(object.dirtiness);
                    }
                    Difficulty::Medium => {
                        object.dirtiness =
                            object.dirtiness.checked_add(1).unwrap_or(object.dirtiness);
                    }
                    Difficulty::Hard => todo!(),
                }
                // make money when mouse button down
                match level_state.difficulty {
                    Difficulty::Easy => {}
                    Difficulty::Medium => {
                        let rep_effect = (level_state.repellants as f32).exp();
                        level_state.money += 0.5 + rep_effect;
                    }
                    Difficulty::Hard => todo!(),
                }
            }
        } else {
            // make money when mouse button not down
            match level_state.difficulty {
                Difficulty::Easy => {
                    level_state.money += 0.1;
                }
                Difficulty::Medium => {}
                Difficulty::Hard => todo!(),
            }
            // level_state.money += match level_state.difficulty {
            //     Difficulty::Easy => 0.1,
            //     Difficulty::Medium => 0.1,
            //     Difficulty::Hard => todo!(),
            // };
        }

        num_objects += 1;
        match level_state.difficulty {
            Difficulty::Easy => {
                if object.dirtiness > MAX_DIRTINESS {
                    objects_complete_dirty += 1;
                }
            }
            Difficulty::Medium => {
                if object.dirtiness > MAX_DIRTINESS || object.dirtiness < 40 {
                    objects_complete_dirty += 1;
                }
            }
            Difficulty::Hard => todo!(),
        }
    }

    //draw money
    draw_text(
        &format!("Money: ${:.2}", level_state.money),
        10.0,
        20.0,
        20.0,
        WHITE,
    );
    draw_text(
        &format!("Goal: ${}", level_state.money_goal),
        10.0,
        40.0,
        20.0,
        WHITE,
    );

    // shift duration (medium level)
    if level_state.difficulty == Difficulty::Medium && duration.is_some() {
        draw_text(
            &format!(
                "Shift ends in: {:.3}s",
                (MEDIUM_DURATION - duration.unwrap()) as f32 / 60.0
            ),
            10.0,
            60.0,
            20.0,
            WHITE,
        );
    }

    //draw buy-repellant/drone button
    let rep_btn_width = 170.0;
    let rep_btn_height = 30.0;
    let rep_btn_top = LineSegment::new((960.0 - rep_btn_width - 10.0, 10.0), (960.0 - 10.0, 10.0));
    let rep_btn_btm = LineSegment::new(
        (960.0 - rep_btn_width - 10.0, rep_btn_height + 10.0),
        (960.0 - 10.0, rep_btn_height + 10.0),
    );
    draw_rectangle(
        rep_btn_top.start.x,
        rep_btn_top.start.y,
        rep_btn_width,
        rep_btn_height,
        BLACK,
    );
    draw_text(
        &format!(
            "{} ${:.2}",
            level_state.repellant_name, level_state.repellant_price
        ),
        rep_btn_top.start.x + 10.0,
        rep_btn_top.start.y + 20.0,
        20.0,
        WHITE,
    );
    draw_text(
        &format!("Owned: [{}]", level_state.repellants),
        rep_btn_top.start.x + 10.0,
        rep_btn_top.start.y + 50.0,
        20.0,
        WHITE,
    );

    // buying supplements
    if is_mouse_button_pressed(MouseButton::Left)
        && rep_btn_top.lies_between(&rep_btn_btm, mouse_pos.into())
        && level_state.money > level_state.repellant_price
    {
        level_state.repellants += 1;
        level_state.money -= level_state.repellant_price;
    }

    // change in state
    if level_state.money > level_state.money_goal {
        // You Won
        Some(Screen::Dialog(Dialog::Won(Stage::A1, Stage::B1)))
    } else if level_state.difficulty == Difficulty::Easy && num_objects == objects_complete_dirty {
        // You Lost
        // Some(Screen::DeathAnim(Stage::A1, 0))
        Some(Screen::Dialog(Dialog::Lost(Stage::B1)))
    } else if level_state.difficulty == Difficulty::Medium
        && (objects_complete_dirty >= 1 || duration.filter(|d| *d > MEDIUM_DURATION).is_some())
    {
        Some(Screen::Dialog(Dialog::Lost(Stage::B1)))
    } else {
        None
    }
}

impl DirtyObj {
    fn new(
        dirtiness: u8,
        start: LineSegment,
        end: LineSegment,
        amount: usize,
        chance_bidir: u8,
    ) -> Self {
        let distance =
            ((end.start + end.end) / vec2(2.0, 2.0)) - ((start.start + start.end) / vec2(2.0, 2.0));
        DirtyObj {
            dirtiness,
            start,
            end,
            amount,
            distance,
            chance_bidir,
        }
    }
}

fn lerp_ant(
    tick: usize,
    object: &DirtyObj,
    mut start: Vec2,
    mut end: Vec2,
    seed: usize,
    randomize_end: bool,
) -> Option<(f32, f32)> {
    if (f32::powi(NOISE[seed % 100], 2) * object.dirtiness as f32) < 63.0 {
        return None;
    }
    if NOISE[(seed + 1) % 100] * 255.0 < object.chance_bidir as f32 {
        std::mem::swap(&mut start, &mut end);
    }
    if randomize_end {
        end = end
            + vec2(
                (NOISE[(seed + 20) % 100] * 2.0 - 1.0) * STOE_SHIFT * object.distance.y,
                (NOISE[(seed + 30) % 100] * 2.0 - 1.0) * STOE_SHIFT * object.distance.x,
            );
    }

    let tick_x = tick + (NOISE[seed % 100] * 20.0 * object.amount as f32) as usize;
    let tick_y = tick + (NOISE[seed % 100] * 20.0 * object.amount as f32) as usize;

    let dist_x = end.x - start.x;
    let dist_y = end.y - start.y;

    let x = start.x
        + (((tick_x as f32 * 0.002) * (end.x - start.x))
            % if dist_x == 0.0 { 1.0 } else { dist_x });
    let y = start.y
        + (((tick_y as f32 * 0.002) * (end.y - start.y))
            % if dist_y == 0.0 { 1.0 } else { dist_y });
    return Some((x, y));
}

#[macroquad::main(conf)]
async fn main() {
    let mut tick: usize = 0;
    let mut death_particles: Option<Emitter> = None;
    let mut duration = None;

    // resources
    let resources = Resources::from([
        (
            ResourceName::ImageA1,
            load_texture("./gimp/bg_a_1.png").await.unwrap(),
        ),
        (
            ResourceName::ImageB1,
            load_texture("./gimp/bg_b_1.png").await.unwrap(),
        ),
        (
            ResourceName::Ant,
            load_texture("./gimp/ant2.png").await.unwrap(),
        ),
        (
            ResourceName::AntCrit,
            load_texture("./gimp/ant_crit.png").await.unwrap(),
        ),
        (
            ResourceName::AntSV,
            load_texture("./gimp/ant3.png").await.unwrap(),
        ),
        (
            ResourceName::AntSVCrit,
            load_texture("./gimp/ant3_crit.png").await.unwrap(),
        ),
        (
            ResourceName::AntSH,
            load_texture("./gimp/ant4.png").await.unwrap(),
        ),
        (
            ResourceName::AntSHCrit,
            load_texture("./gimp/ant3_crit.png").await.unwrap(),
        ),
    ]);

    let levels = HashMap::from([
        // level 1: killing ants
        (
            Stage::A1,
            LevelState {
                difficulty: Difficulty::Easy,
                money: 0.0,
                money_goal: 1_000.0,
                repellants: 0,
                repellant_name: "Repellant".to_owned(),
                repellant_price: 4.0,
                scene: ResourceName::ImageA1,
                objects: vec![
                    // coffee cup
                    DirtyObj::new(
                        64,
                        LineSegment::new((179.0, 412.0), (223.0, 412.0)),
                        LineSegment::new((186.0, 360.0), (215.0, 361.0)),
                        200,
                        0,
                    ),
                    // keyboard
                    DirtyObj::new(
                        64,
                        LineSegment::new((250.0, 404.0), (590.0, 404.0)),
                        LineSegment::new((278.0, 361.0), (575.0, 361.0)),
                        700,
                        128,
                    ),
                    // mouse
                    DirtyObj::new(
                        64,
                        LineSegment::new((624.0, 369.0), (636.0, 391.0)),
                        LineSegment::new((652.0, 355.0), (664.0, 378.0)),
                        90,
                        63,
                    ),
                ],
            },
        ),
        (
            Stage::B1,
            LevelState {
                difficulty: Difficulty::Medium,
                money: 0.0,
                money_goal: 10_000.0,
                repellants: 1,
                repellant_name: "Drones".to_owned(),
                repellant_price: 100.0,
                scene: ResourceName::ImageB1,
                objects: vec![
                    // house 1
                    DirtyObj::new(
                        163,
                        LineSegment::new((332.0, 126.0), (404.0, 126.0)),
                        LineSegment::new((332.0, 178.0), (404.0, 178.0)),
                        500,
                        63,
                    ),
                    // house 2
                    DirtyObj::new(
                        162,
                        LineSegment::new((563.0, 201.0), (563.0, 257.0)),
                        LineSegment::new((673.0, 260.0), (673.0, 198.0)),
                        800,
                        191,
                    ),
                    // house 3
                    DirtyObj::new(
                        163,
                        LineSegment::new((332.0, 273.0), (404.0, 273.0)),
                        LineSegment::new((332.0, 325.0), (404.0, 325.0)),
                        500,
                        225,
                    ),
                ],
            },
        ),
    ]);

    // levels
    let mut state = GameState {
        // screen: Screen::DeathAnim(Stage::A1, 0),
        screen: Screen::Stage(Stage::B1, levels.get(&Stage::B1).unwrap().clone()),
        levels,
    };

    loop {
        clear_background(BLACK);

        let next_screen = match &mut state.screen {
            Screen::MainMenu => {
                if is_mouse_button_pressed(MouseButton::Left) {
                    Some(Screen::Stage(
                        Stage::A1,
                        state.levels.get(&Stage::A1).unwrap().clone(),
                    ))
                } else {
                    None
                }
            }
            Screen::Dialog(dialog) => match dialog {
                Dialog::Won(stage, next_stage) => {
                    draw_text(&format!("You won stage {stage}"), 10.0, 20.0, 20.0, WHITE);
                    if is_mouse_button_pressed(MouseButton::Left) {
                        Some(Screen::Stage(
                            *next_stage,
                            state.levels.get(next_stage).unwrap().clone(),
                        ))
                    } else {
                        None
                    }
                }
                Dialog::Lost(stage) => {
                    draw_text(&format!("You lost stage {stage}"), 10.0, 20.0, 20.0, WHITE);
                    if is_mouse_button_pressed(MouseButton::Left) {
                        Some(Screen::Stage(
                            *stage,
                            state.levels.get(stage).unwrap().clone(),
                        ))
                    } else {
                        None
                    }
                }
            },
            Screen::Stage(stage, ref mut level_state) => {
                manage_level(level_state, &resources, tick, &mut duration)
            }
            Screen::DeathAnim(stage, progress) => {
                let next_screen;
                draw_texture(
                    resources
                        .get(&state.levels.get(&stage).unwrap().scene)
                        .unwrap(),
                    0.0,
                    0.0,
                    WHITE,
                );

                if let Some(particles) = &mut death_particles {
                    particles.draw((480.0, 540.0).into());
                    if *progress > 253 {
                        particles.config.emitting = false;
                        death_particles = None;

                        next_screen = Some(Screen::Dialog(Dialog::Lost(*stage)));
                    } else {
                        *progress += 2;
                        particles.config.amount += 10;

                        next_screen = None;
                    }
                } else {
                    death_particles = Some(Emitter::new(EmitterConfig {
                        emission_shape: EmissionShape::Rect {
                            width: 960.0,
                            height: 0.0,
                        },
                        texture: Some(resources.get(&ResourceName::Ant).unwrap().clone()),
                        lifetime: 2.3,
                        amount: 10,
                        initial_direction_spread: 0.,
                        initial_velocity: 300.0,
                        // atlas: Some(AtlasConfig::new(4, 4, 8..)),
                        size: 8.0,
                        // blend_mode: BlendMode::Additive,
                        ..Default::default()
                    }));
                    *progress = 0;
                    next_screen = None;
                }

                /*
                let screen_top = LineSegment::new((0.0, 0.0), (960.0, 0.0));
                let screen_btm = LineSegment::new((0.0, 540.0), (960.0, 540.0));
                for (idx, (start, end)) in screen_btm
                    .points_on(20000)
                    .zip(screen_top.points_on(20000))
                    .enumerate()
                {
                    let pos = lerp_ant(
                        tick,
                        &DirtyObj {
                            dirtiness: MAX_DIRTINESS,
                            start: screen_btm.clone(),
                            end: screen_top.clone(),
                            amount: 1_000_000,
                            distance: (0.0, 540.0).into(),
                            chance_bidir: 0,
                        },
                        start,
                        end,
                        idx,
                        true,
                    );
                    if let Some((x, y)) = pos {
                        println!("{},{}", x, y);
                        draw_texture(resources.get(&ResourceName::Ant).unwrap(), x, y, WHITE);
                    }
                }
                */
                draw_text(&format!("You lost stage {stage}"), 10.0, 20.0, 20.0, WHITE);
                // if is_mouse_button_pressed(MouseButton::Left) {
                //     Some(Screen::Stage(
                //         *stage,
                //         state.levels.get(stage).unwrap().clone(),
                //     ))
                // } else {
                //     None
                // }
                next_screen
            }
        };
        if let Some(next_screen) = next_screen {
            state.screen = next_screen;
        }

        // draw_rectangle(179.0, 412.0, 50.0, 50.0, WHITE);
        // let _ = draw_text("You Won!\nYou Lost!", 179.0, 412.0, 40.0, WHITE);

        tick += 1;
        next_frame().await;
    }
}
