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
