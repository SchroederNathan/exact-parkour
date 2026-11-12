//! The two bodies of `art/`, which the player chooses between at the start.
use exact_beat_em_up_logic::{body, stickman, ExactBeatEmUp, Options, LOCKER_MODE};
use exact_game::asset::Model;
use exact_game::{Mat4, Pose, Sim, Vec3};

type Game = Sim<ExactBeatEmUp>;

fn start(character: u32) -> Game {
    let mut game: Game = exact_game::sim_with_art!(Options {
        seed: 7,
        character,
        ..Options::default()
    })
    .unwrap();
    game.run(25.0);
    game
}

// Where each node of `model` rests, in the model's space, by its name.
fn rest(model: &Model) -> std::collections::BTreeMap<String, Vec3> {
    let mut places = std::collections::BTreeMap::new();
    for node in &model.nodes {
        // A parent is not always before its child in a baked model.
        let mut total = Mat4::from_cols_array(&node.transform);
        let mut up = node.parent;
        while let Some(parent) = up {
            total = Mat4::from_cols_array(&model.nodes[parent as usize].transform) * total;
            up = model.nodes[parent as usize].parent;
        }
        places.insert(node.name.clone(), total.w_axis.truncate());
    }
    places
}

#[test]
fn each_body_of_art_has_the_joints_of_the_rig_at_the_places_of_the_rig() {
    let game = start(0);
    let rig = stickman::rig(body::HEIGHT);
    for name in stickman::BODIES {
        let model = game.world().model(name).unwrap_or_else(|| panic!("{name} is not baked"));
        let places = rest(model);
        for bone in rig.bones() {
            let at = places
                .get(&bone.name)
                .unwrap_or_else(|| panic!("{name} has no joint `{}`", bone.name));
            assert!(
                (*at - bone.head).length() < 1e-4,
                "{name}: `{}` rests at {at}, the rig's at {}",
                bone.name,
                bone.head
            );
        }
        // What a Blender export carries that changes the picture.
        for (index, material) in model.materials.iter().enumerate() {
            assert!(!material.double_sided, "{name}: material {index} is double-sided");
            assert!(material.base_color_texture.is_some(), "{name}: material {index} has no texture");
        }
        for mesh in &model.meshes {
            assert!(mesh.colors.is_empty(), "{name} has vertex colours");
        }
    }
}

#[test]
fn a_dressed_body_is_one_mesh_that_the_rig_moves() {
    let game = start(0);
    let rig = stickman::rig(body::HEIGHT);
    for name in stickman::BODIES {
        let body = game.world().model(name).unwrap();
        let model = stickman::dressed(&rig, body).unwrap();
        model.validate().unwrap();
        assert_eq!(model.meshes.len(), 1);
        let mesh = &model.meshes[0];
        let vertices = mesh.positions.len() / 3;
        // A budget for phones, beside the level.
        assert!(vertices < 15_000 && mesh.indices.len() / 3 < 20_000, "{name}: {vertices} vertices");
        for vertex in 0..vertices {
            let weights = &mesh.weights[vertex * 4..vertex * 4 + 4];
            assert!((weights.iter().sum::<f32>() - 1.0).abs() < 1e-3);
            for (&joint, &weight) in mesh.joints[vertex * 4..vertex * 4 + 4].iter().zip(weights) {
                assert!(weight >= 0.0 && (joint as usize) < rig.bones().len());
                // The root moves the whole body. No part of a body hangs from it.
                assert!(joint != 0 || weight == 0.0, "{name}: a vertex follows the root");
            }
        }
        // It stands on the ground at the mannequin's height and inside the capsule's width.
        assert!(model.bounds[1] > -0.005 && (model.bounds[4] - body::HEIGHT).abs() < 0.005);
        assert!(model.bounds[0] > -0.35 && model.bounds[3] < 0.35);
        // The same joints and clips as the mannequin, so every move plays.
        let mannequin = stickman::model(&rig);
        assert_eq!(model.clips.len(), mannequin.clips.len());
        assert_eq!(model.nodes.len(), mannequin.nodes.len());
    }
}

#[test]
fn the_player_runs_the_same_course_in_each_body() {
    let course = |character: u32| {
        let mut game = start(character);
        game.key_down("KeyW");
        game.run(1500.0);
        let at = game.world().require::<exact_game::Transform>("player").position;
        let pose = game.world().get::<Pose>("stickman").unwrap().local.clone();
        (at, pose)
    };
    let mannequin = course(0);
    for character in 1..=stickman::BODIES.len() as u32 {
        let dressed = course(character);
        assert_eq!(dressed.0, mannequin.0, "body {character} went somewhere else");
        assert_eq!(dressed.1, mannequin.1, "body {character} moved its joints differently");
    }
}

#[test]
fn the_locker_keeps_the_body_in_place_and_takes_the_eye_round_it() {
    let mut game: Game = exact_game::sim_with_art!(Options {
        seed: 3,
        character: 2,
        mode: LOCKER_MODE,
        ..Options::default()
    })
    .unwrap();
    let place = |game: &Game, name: &str| game.world().require::<exact_game::Transform>(name).position;
    game.run(25.0);
    let (stood, eye) = (place(&game, "player"), place(&game, "camera"));
    // The eye starts in front of the body, which faces south there.
    assert!(eye.z - stood.z > 1.0, "the eye is at {eye}, the body at {stood}");
    // A key that moves the body in a level is not pressed here: the start screen has the focus.
    game.run(3000.0);
    let (stands, round) = (place(&game, "player"), place(&game, "camera"));
    assert!((stands - stood).length() < 1e-3, "the body went from {stood} to {stands}");
    assert!((round - eye).length() > 1.0, "the eye stayed at {eye}");
    let reach = |eye: Vec3| (eye - stood).length();
    assert!((reach(round) - reach(eye)).abs() < 0.05, "the eye came closer or went away");
}

// How much of vertex `vertex` of `mesh` follows the joint numbered `joint`.
fn follows(mesh: &exact_game::asset::MeshData, vertex: usize, joint: usize) -> f32 {
    (0..4)
        .filter(|slot| mesh.joints[vertex * 4 + slot] as usize == joint)
        .map(|slot| mesh.weights[vertex * 4 + slot])
        .sum()
}

#[test]
fn a_foot_is_one_piece_and_the_neck_turns_about_its_own_joint() {
    let game = start(0);
    let rig = stickman::rig(body::HEIGHT);
    let joint = |name: &str| rig.bones().iter().position(|bone| bone.name == name).unwrap();
    for name in stickman::BODIES {
        let model = stickman::dressed(&rig, game.world().model(name).unwrap()).unwrap();
        let mesh = &model.meshes[0];
        let (mut sole, mut neck, mut column) = (0, 0.0, 0);
        for vertex in 0..mesh.positions.len() / 3 {
            let at = Vec3::from_slice(&mesh.positions[vertex * 3..]);
            // Under the ankle every vertex follows the foot, the heel too. A heel
            // that the shin holds comes apart from the foot when the foot turns.
            if at.y < 0.05 {
                let foot = joint(if at.x > 0.0 { "foot_l" } else { "foot_r" });
                assert!(
                    follows(mesh, vertex, foot) > 0.95,
                    "{name}: a vertex of the sole at {at} follows the foot by {}",
                    follows(mesh, vertex, foot)
                );
                sole += 1;
            }
            // The column of the neck, under the jaw, follows the neck and not the
            // head: the head turns about a joint that is 20 cm higher.
            if at.y > 1.5 && at.y < 1.55 && at.x.abs() < 0.05 && at.z.abs() < 0.05 {
                neck += follows(mesh, vertex, joint("neck"));
                column += 1;
            }
        }
        assert!(sole > 100, "{name}: {sole} vertices in the soles");
        assert!(column > 0 && neck / column as f32 > 0.6, "{name}: the neck follows its joint by {}", neck / column as f32);
    }
}

#[test]
fn a_stand_is_the_same_on_both_sides_and_the_hands_are_by_the_thighs() {
    use exact_game::Quat;
    let rig = stickman::rig(body::HEIGHT);
    let joint = |name: &str| rig.bones().iter().position(|bone| bone.name == name).unwrap();
    for character in 0..=stickman::BODIES.len() as u32 {
        let mut game = start(character);
        game.run(3000.0);
        let pose = game.world().get::<Pose>("stickman").unwrap().local.clone();
        let turn = |name: &str| Quat::from_slice(&pose[joint(name) * 10 + 3..joint(name) * 10 + 7]);
        for part in ["upper_arm", "forearm", "hand", "thigh", "shin", "foot"] {
            let (left, right) = (turn(&format!("{part}_l")), turn(&format!("{part}_r")));
            // The same turn on the other side of the body.
            let mirrored = Quat::from_xyzw(right.x, -right.y, -right.z, right.w);
            assert!(
                left.angle_between(mirrored) < 1e-3,
                "character {character}: {part} is at {left} on the left and {right} on the right"
            );
        }
        if character > 0 {
            let wrist = exact_beat_em_up_logic::hold::joint(game.world(), "hand_l");
            let hips = exact_beat_em_up_logic::hold::joint(game.world(), "hips");
            let aside = Vec3::new(wrist.x - hips.x, 0.0, wrist.z - hips.z).length();
            assert!((0.18..0.25).contains(&aside), "character {character}: the wrist is {aside} m from the middle");
        }
    }
}

// Every joint of the drawn body through every move, tick by tick.
mod fluid {
    use super::*;
    use exact_beat_em_up_logic::body::Parkour;
    use exact_beat_em_up_logic::{debug, park};
    use exact_game::animation::joint_matrix;
    use exact_game::Transform;
    use exact_game_physics::CapsuleController;

    const TICK_MS: f64 = 8.334;

    /// The most that a joint went in one tick, and the most that its step changed
    /// from one tick to the next, both in the body's own frame, metres. With them
    /// the joint, the tick and what the body did then.
    #[derive(Default)]
    pub struct Worst {
        pub step: (f32, String),
        pub change: (f32, String),
    }

    pub struct Watch {
        pub game: Game,
        names: Vec<String>,
        before: Vec<Vec3>,
        step: Vec<Vec3>,
        known: u32,
        tick: usize,
        pub worst: Worst,
    }

    impl Watch {
        pub fn new(character: u32) -> Self {
            let mut game = start(character);
            // No roller comes down the ramp into the scene.
            let w = game.world_mut();
            let rollers: Vec<_> = w.query::<&park::Roller>().iter().map(|(e, _)| e).collect();
            for roller in rollers {
                w.despawn(roller);
            }
            w.resource_mut::<park::Ramp>().released = u32::MAX;
            let names = stickman::rig(body::HEIGHT).bones().iter().map(|b| b.name.clone()).collect();
            Watch { game, names, before: vec![], step: vec![], known: 0, tick: 0, worst: Worst::default() }
        }

        /// Stand the body with its feet at `at`, facing `yaw` radians. What the
        /// joints did before is forgotten: the body was put there, it did not go.
        pub fn place(&mut self, at: Vec3, yaw: f32) {
            let w = self.game.world_mut();
            w.require_mut::<Transform>("player").position = at + Vec3::Y * (body::HALF + 0.02);
            *w.require_mut::<Parkour>("player") = Parkour { yaw, ..Parkour::default() };
            let mut capsule = w.require_mut::<CapsuleController>("player");
            capsule.velocity = Vec3::ZERO;
            capsule.grounded = false;
            capsule.support = None;
            drop(capsule);
            self.known = 0;
            // The pose settles from the placing before it is watched.
            for _ in 0..30 {
                self.game.run(TICK_MS);
            }
        }

        /// Hold the move stick part of the way out, as `debug`'s `stick` does.
        pub fn stick(&mut self, x: f32, y: f32) {
            *self.game.world_mut().resource_mut::<debug::Stick>() =
                debug::Stick { held: x != 0.0 || y != 0.0, x, y };
        }

        fn joints(&self) -> Vec<Vec3> {
            let world = self.game.world();
            let model = world.model(stickman::MODEL).unwrap();
            let pose = world.get::<Pose>("stickman").unwrap();
            (0..self.names.len())
                .map(|joint| joint_matrix(model, &pose.local, joint as u32).w_axis.truncate())
                .collect()
        }

        pub fn doing(&self) -> String {
            let action = format!("{:?}", self.game.world().require::<Parkour>("player").action);
            action.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("").to_string()
        }

        /// Run `ms` of the game and watch every joint on every tick.
        pub fn run(&mut self, ms: f64) {
            for _ in 0..(ms / TICK_MS).round() as usize {
                self.game.run(TICK_MS);
                self.tick += 1;
                let doing = self.doing();
                // A ragdoll is not a pose: the physics moves its points.
                if doing == "Ragdoll" {
                    self.known = 0;
                    continue;
                }
                let now = self.joints();
                let step: Vec<Vec3> = if self.known >= 1 {
                    now.iter().zip(&self.before).map(|(a, b)| *a - *b).collect()
                } else {
                    vec![]
                };
                for (joint, moved) in step.iter().enumerate() {
                    let label = || format!("{} at tick {} in {doing}", self.names[joint], self.tick);
                    if moved.length() > self.worst.step.0 {
                        self.worst.step = (moved.length(), label());
                    }
                    if self.known >= 2 {
                        let change = (*moved - self.step[joint]).length();
                        if change > self.worst.change.0 {
                            self.worst.change = (change, label());
                        }
                    }
                }
                // FLUID_JOINT=shin_l FLUID_FROM=380 FLUID_TO=400 prints that joint's place.
                if let Ok(joint) = std::env::var("FLUID_JOINT") {
                    let number = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<usize>().ok());
                    if (number("FLUID_FROM").unwrap_or(0)..=number("FLUID_TO").unwrap_or(usize::MAX)).contains(&self.tick) {
                        let world = self.game.world();
                        let body = world.require::<Parkour>("player");
                        let speed = world.require::<CapsuleController>("player").velocity;
                        for part in joint.split(',') {
                            let index = self.names.iter().position(|n| n == part).unwrap();
                            print!("{part} {:.3} {:.3} {:.3}  ", now[index].x, now[index].y, now[index].z);
                        }
                        println!("| tick {} {doing} tilt {:.3} speed {:.2} {:.2}", self.tick, body.tilt.length(), Vec3::new(speed.x, 0.0, speed.z).length(), speed.y);
                    }
                }
                self.before = now;
                self.step = step;
                self.known = (self.known + 1).min(2);
            }
        }

        /// Run until the body does `what`, for `ms` at most. Returns whether it did.
        pub fn until(&mut self, what: &str, ms: f64) -> bool {
            for _ in 0..(ms / TICK_MS).round() as usize {
                if self.doing() == what {
                    return true;
                }
                self.run(TICK_MS);
            }
            self.doing() == what
        }
    }
}

// One scene of the body's moves: what it is called, and the keys of it.
type Scene = (&'static str, fn(&mut fluid::Watch));

const NORTH: f32 = std::f32::consts::PI;

fn scenes() -> Vec<Scene> {
    let scenes: Vec<Scene> = vec![
        ("stand, walk, run, sprint and stop", |s: &mut fluid::Watch| {
            s.place(Vec3::new(-6.0, 0.0, 26.0), NORTH);
            s.run(700.0);
            s.stick(0.0, 0.25);
            s.run(1500.0);
            s.stick(0.0, 0.5);
            s.run(1500.0);
            s.stick(0.0, 0.0);
            s.game.key_down("KeyW");
            s.run(1500.0);
            s.game.key_down("ShiftLeft");
            s.run(1500.0);
            s.game.key_up("ShiftLeft");
            s.run(1000.0);
            s.game.key_up("KeyW");
            s.run(1200.0);
        }),
        ("a stop out of a sprint, and an about-face", |s: &mut fluid::Watch| {
            s.place(Vec3::new(-6.0, 0.0, 26.0), NORTH);
            s.game.key_down("ShiftLeft");
            s.game.key_down("KeyW");
            s.run(1500.0);
            s.game.key_up("KeyW");
            s.game.key_up("ShiftLeft");
            s.run(1200.0);
            s.game.key_down("KeyW");
            s.run(1000.0);
            s.game.key_up("KeyW");
            s.game.key_down("KeyS");
            s.run(1500.0);
            s.game.key_up("KeyS");
            s.game.key_down("KeyD");
            s.run(1000.0);
            s.game.key_up("KeyD");
            s.run(800.0);
        }),
        ("jumps from a stand, a run and a sprint", |s: &mut fluid::Watch| {
            s.place(Vec3::new(-6.0, 0.0, 26.0), NORTH);
            s.run(300.0);
            s.game.tap("Space");
            s.run(1300.0);
            s.game.key_down("KeyW");
            s.run(900.0);
            s.game.tap("Space");
            s.run(1300.0);
            s.game.key_down("ShiftLeft");
            s.run(900.0);
            s.game.tap("Space");
            s.run(1400.0);
            s.game.tap("Space");
            s.run(1400.0);
        }),
        ("a slide out of a sprint, into a run", |s: &mut fluid::Watch| {
            s.place(Vec3::new(-6.0, 0.0, 26.0), NORTH);
            s.game.key_down("ShiftLeft");
            s.game.key_down("KeyW");
            s.run(1200.0);
            s.game.tap("KeyC");
            assert!(s.until("Slide", 300.0), "no slide, {}", s.doing());
            s.run(1600.0);
        }),
        ("a vault over a wall", |s: &mut fluid::Watch| {
            s.place(Vec3::new(-2.0, 0.0, -3.4), NORTH);
            s.game.key_down("KeyW");
            s.run(380.0);
            s.game.tap("Space");
            assert!(s.until("Vault", 500.0), "no vault, {}", s.doing());
            s.run(1500.0);
        }),
        ("a climb onto a deep wall", |s: &mut fluid::Watch| {
            s.place(Vec3::new(6.5, 0.0, -3.4), NORTH);
            s.game.key_down("KeyW");
            s.run(380.0);
            s.game.tap("Space");
            assert!(s.until("Climb", 500.0), "no climb, {}", s.doing());
            s.run(1800.0);
        }),
        ("a catch, a hang, a shimmy and a climb", |s: &mut fluid::Watch| {
            s.place(Vec3::new(-3.0, 0.0, -10.3), NORTH);
            s.game.key_down("KeyW");
            s.run(260.0);
            s.game.tap("Space");
            assert!(s.until("Hang", 900.0), "no hang, {}", s.doing());
            s.run(900.0);
            s.game.key_up("KeyW");
            s.game.key_down("KeyD");
            s.run(1200.0);
            s.game.key_up("KeyD");
            s.game.key_down("KeyA");
            s.run(900.0);
            s.game.key_up("KeyA");
            s.game.key_down("KeyW");
            s.game.tap("Space");
            assert!(s.until("Climb", 500.0), "no climb, {}", s.doing());
            s.run(1800.0);
        }),
        ("a hang, and a drop from it", |s: &mut fluid::Watch| {
            s.place(Vec3::new(-3.0, 0.0, -10.3), NORTH);
            s.game.key_down("KeyW");
            s.run(260.0);
            s.game.tap("Space");
            assert!(s.until("Hang", 900.0), "no hang, {}", s.doing());
            s.run(600.0);
            s.game.key_up("KeyW");
            s.game.key_down("KeyS");
            s.run(1500.0);
        }),
        ("landings from 0.7, 1.6 and 3.2 m at a stand", |s: &mut fluid::Watch| {
            for height in [0.7, 1.6, 3.2] {
                s.place(Vec3::new(-6.0, 0.0, 22.0), NORTH);
                let w = s.game.world_mut();
                w.require_mut::<exact_game::Transform>("player").position.y += height;
                s.run(2600.0);
            }
        }),
        ("landings from 1.6 and 3.2 m at a run", |s: &mut fluid::Watch| {
            for height in [1.6, 3.2] {
                s.place(Vec3::new(-6.0, 0.0, 26.0), NORTH);
                s.game.key_down("KeyW");
                s.run(700.0);
                let w = s.game.world_mut();
                w.require_mut::<exact_game::Transform>("player").position.y += height;
                s.run(2600.0);
                s.game.key_up("KeyW");
                s.run(600.0);
            }
        }),
        ("the stairs, up and down", |s: &mut fluid::Watch| {
            s.place(Vec3::new(6.0, 0.0, 2.6), 0.0);
            s.game.key_down("KeyS");
            s.run(2600.0);
            s.game.key_up("KeyS");
            s.game.key_down("KeyW");
            s.run(2600.0);
        }),
        ("a stand for six seconds", |s: &mut fluid::Watch| {
            s.place(Vec3::new(-6.0, 0.0, 22.0), NORTH);
            s.run(6000.0);
        }),
    ];
    scenes
}

#[test]
fn every_joint_goes_smoothly_through_every_move() {
    // A sprint's knee goes 0.07 m in a tick, and its foot's step changes by
    // 0.04 m when it meets the ground. The head of a body that rolls goes
    // 0.11 m. A joint that is somewhere else at once does several times that:
    // before the limbs were made to go straight gently, a knee went 0.19 m in
    // one tick of a climb and changed its step by 0.16 m at a landing.
    const FURTHEST: f32 = 0.12;
    const SHARPEST: f32 = 0.09;
    let mut failures = vec![];
    for character in [0, 1] {
        for (name, keys) in scenes() {
            if std::env::var("FLUID_SCENE").is_ok_and(|only| !name.contains(&only)) {
                continue;
            }
            let mut watch = fluid::Watch::new(character);
            keys(&mut watch);
            let worst = &watch.worst;
            println!(
                "character {character}, {name}: furthest {:.3} m ({}), sharpest {:.3} m ({})",
                worst.step.0, worst.step.1, worst.change.0, worst.change.1
            );
            if worst.step.0 > FURTHEST {
                failures.push(format!("character {character}, {name}: {} went {} m in a tick", worst.step.1, worst.step.0));
            }
            if worst.change.0 > SHARPEST {
                failures.push(format!("character {character}, {name}: {} changed its step by {} m", worst.change.1, worst.change.0));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
