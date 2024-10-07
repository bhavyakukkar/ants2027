use std::{
    collections::{HashMap, VecDeque},
    fmt,
    slice::Iter,
};

use macroquad::{
    color::*,
    input::{is_mouse_button_down, is_mouse_button_pressed, mouse_position, MouseButton},
    math::{vec2, Vec2},
    rand::gen_range,
    shapes::draw_rectangle,
    text::draw_text,
    texture::{draw_texture, load_texture, Texture2D},
    window::{clear_background, next_frame, Conf},
};

mod util;
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
        window_title: "My Life with Ants in 2027".into(),
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
    Story1,
    Story2,
    Story3,
}
type Resources = HashMap<ResourceName, Texture2D>;

struct GameState {
    screen: Screen,
    levels: HashMap<Stage, LevelState>,
}

enum Screen {
    MainMenu,
    Stage(Stage, LevelState),
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

struct StoryIter {
    pages: VecDeque<(ResourceName, Vec<String>)>,
}

impl StoryIter {
    fn peek(&self) -> Option<(ResourceName, Vec<String>)> {
        match self.pages.len() {
            0 => None,
            _ => Some(self.pages.get(0).unwrap().clone()),
        }
    }
}

impl Iterator for StoryIter {
    type Item = (ResourceName, Vec<String>);

    fn next(&mut self) -> Option<Self::Item> {
        self.pages.pop_front()
    }
}

enum Dialog {
    Lost(Stage),
    Won(Stage, Stage),
    Story(StoryIter, Stage),
    Thanks,
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
    static MEDIUM_DURATION: usize = 4 * 60 * 60;
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
            };

            let (x, y) = match lerp_ant(
                tick,
                &object,
                start,
                end,
                idx,
                match level_state.difficulty {
                    Difficulty::Easy => true,
                    Difficulty::Medium => false,
                },
            ) {
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
        };

        if update_dirtiness {
            object.dirtiness = match level_state.difficulty {
                Difficulty::Easy => object.dirtiness.checked_add(1).unwrap_or(object.dirtiness),
                Difficulty::Medium => object.dirtiness.checked_sub(1).unwrap_or(object.dirtiness),
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
                }
                // make money when mouse button down
                match level_state.difficulty {
                    Difficulty::Easy => {}
                    Difficulty::Medium => {
                        let rep_effect = 2.0 * (level_state.repellants as f32).ln();
                        level_state.money += 0.5 + rep_effect;
                    }
                }
            }
        } else {
            // make money when mouse button not down
            match level_state.difficulty {
                Difficulty::Easy => {
                    level_state.money += 0.1;
                }
                Difficulty::Medium => {}
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
        // Some(Screen::Dialog(Dialog::Won(Stage::A1, Stage::B1)))
        match level_state.difficulty {
            Difficulty::Easy => 
        Some(Screen::Dialog(Dialog::Story(StoryIter{
            pages: vec![
                (Story3, vec![
                    "well done, you did it. you earned enough money to move to the center.".to_owned()]
                ),
                (Story3, vec![
                    "you are now ant free. this feels like heaven.".to_owned()]
                ),
                (Story3, vec![
                    "the center has some next-generation technology that makes the ants pass out".to_owned(),
                    "for very long durations of time.".to_owned()]
                ),
                (Story3, vec![
                    "they are still up in their labs looking for ways to kill an ant, i hear.".to_owned()]
                ),
                (Story3, vec![
                    "however, the luxury has changed you.".to_owned()]
                ),
                (Story3, vec![
                    "made you afraid of losing it.".to_owned()]
                ),
                (Story3, vec![
                    "your new job pays very handsomly and the better you do your job,".to_owned(),
                    "the more years you secure this life.".to_owned()]
                ),
                (Story3, vec![
                    "you are loyal to the work you do and the people who pay you for it.".to_owned()]
                ),
                (Story3, vec![
                    "you collect all the ants rendered unconscious by the machine and".to_owned(),
                    "sneakily dump them outside for a living!".to_owned()]
                ),
                (Story3, vec![
                    "the toxic substance used by the machine is not fit to be touched".to_owned(),
                    "so you get to use drones to go deliver the bags of ants for you.".to_owned()]
                ),
                (Story3, vec![
                    "you're in charge of a small street with only 3 inhabited houses.".to_owned()]
                ),
                (Story3, vec![
                    "your job involves being careful that you go unnoticed, and this includes".to_owned(),
                    "not making any of the residents suspicious.".to_owned()]
                ),
                (Story3, vec![
                    "unloading too many ants as well as not unloading enough will bring suspicion.".to_owned()]
                ),
                (Story3, vec![
                    "your shift starts as soon as the shift before you ends. get ready".to_owned()]
                ),
            ].into()
        }, Stage::B1))),
            Difficulty::Medium => Some(Screen::Dialog(Dialog::Thanks)),
        }
    } else if level_state.difficulty == Difficulty::Easy && num_objects == objects_complete_dirty {
        // You Lost
        // Some(Screen::DeathAnim(Stage::A1, 0))
        Some(Screen::Dialog(Dialog::Lost(Stage::A1)))
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
    use ResourceName::*;
    let mut tick: usize = 0;
    let mut duration = None;

    let story1 = StoryIter{pages: vec![
        (Story2, vec![
            "the year is 2027. you had big expectations of this year back in 2024.".to_owned()
        ]),
        (Story2, vec![
            "instead, you get an ant infestation epidemic. ants have gotten a lot more adaptive.".to_owned()
        ]),
        (Story2, vec![
            "a person living with upper-class income can afford to avoid ants from infesting any".to_owned(),
            "dust, food or sweat for 5 minutes.".to_owned()
        ]),
        (Story2, vec![
            "I, on the other hand have to compromise for a grand 5 seconds.".to_owned()
        ]),
        (Story2, vec![
            "the only way out from this anguish is to buy a place at the centre.".to_owned()
        ]),
        (Story2, vec![
            "i have lots of clients in need of a website to advertise their".to_owned(),
            "ant-repellant products.".to_owned()
        ]),
        (Story2, vec![
            "all i need is to survive".to_owned()
        ]),
        (Story2, vec![
            "its difficult to have hope in these times but a little energy and a little strategy".to_owned(),
            "and i may be able to make it out.".to_owned()
        ]),
        (Story2, vec![
            "i can buy repellant to slow down how fast these bad boys multiply.".to_owned(),
        ]),
        (Story2, vec![
            "as long as they dont filth all of my belongings, ill make it.".to_owned(),
        ]),
        (Story2, vec![
            "i make money every second that i am not busy tending to ants.".to_owned()
        ]),
    ].into()};

    // resources
    #[rustfmt::skip]
    let resources = Resources::from([
        (ImageA1, load_texture("./gimp/bg_a_1.png").await.unwrap()),
        (ImageB1, load_texture("./gimp/bg_b_1.png").await.unwrap()),
        (Ant, load_texture("./gimp/ant2.png").await.unwrap()),
        (AntCrit, load_texture("./gimp/ant_crit.png").await.unwrap()),
        (AntSV, load_texture("./gimp/ant3.png").await.unwrap()),
        (AntSVCrit, load_texture("./gimp/ant3_crit.png").await.unwrap()),
        (AntSH, load_texture("./gimp/ant4.png").await.unwrap()),
        (AntSHCrit, load_texture("./gimp/ant3_crit.png").await.unwrap()),
        (Story1, load_texture("./gimp/story3.png").await.unwrap()),
        (Story2, load_texture("./gimp/story1.png").await.unwrap()),
        (Story3, load_texture("./gimp/story3.png").await.unwrap()),
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
                        LineSegment::new((674.0, 196.0), (674.0, 262.0)),
                        LineSegment::new((614.0, 220.0), (614.0, 222.0)),
                        800,
                        0,
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
        // screen: Screen::Stage(Stage::B1, levels.get(&Stage::B1).unwrap().clone()),
        screen: Screen::Dialog(Dialog::Story(story1, Stage::A1)),
        // screen: Screen::Dialog(Dialog::Thanks),
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
                    draw_text(match stage {
                        Stage::A1 => "You lost. There were two many ants. You died a disgusting death. Click to try again.",
                        Stage::B1 => "You lost. The people got suspicious and you were fired from your job. Click to try again."
                    }, 10.0, 20.0, 20.0, WHITE);
                    if is_mouse_button_pressed(MouseButton::Left) {
                        Some(Screen::Stage(
                            *stage,
                            state.levels.get(stage).unwrap().clone(),
                        ))
                    } else {
                        None
                    }
                }
                Dialog::Story(story_iter, next_stage) => {
                    if let Some((resource_name, page)) = story_iter.peek() {
                        draw_texture(resources.get(&resource_name).unwrap(), 0.0, 0.0, WHITE);
                        for (idx, line) in page.iter().enumerate() {
                            draw_text(&line, 30.0, 40.0 + idx as f32 * 30.0, 25.0, WHITE);
                        }
                        if is_mouse_button_pressed(MouseButton::Left) {
                            let _ = story_iter.next();
                            None
                        } else {
                            None
                        }
                    } else {
                        Some(Screen::Stage(
                            *next_stage,
                            state.levels.get(next_stage).unwrap().clone(),
                        ))
                    }
                }
                Dialog::Thanks => {
                    draw_texture(resources.get(&Story2).unwrap(), 0.0, 0.0, WHITE);
                    draw_text("You had a great shift.", 60.0, 80.0, 35.0, WHITE);
                    draw_text("Thanks for playing.", 60.0, 120.0, 35.0, WHITE);
                    draw_text("Made by nigel", 60.0, 200.0, 30.0, WHITE);
                    None
                }
            },
            Screen::Stage(_stage, ref mut level_state) => {
                manage_level(level_state, &resources, tick, &mut duration)
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
