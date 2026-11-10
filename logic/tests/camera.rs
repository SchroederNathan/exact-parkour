use exact_beat_em_up_logic::{camera::Orbit, park, ExactBeatEmUp, Options, ARENA_MODE};
use exact_game::*;
use exact_game_physics::{self as physics, Collider, Shape};

fn start() -> Sim<ExactBeatEmUp> {
    let mut sim = Sim::new(Options {
        seed: 7,
        mode: ARENA_MODE,
        ..Options::default()
    })
    .unwrap();
    sim.run(25.0);
    sim
}

fn pointer(sim: &mut Sim<ExactBeatEmUp>, phase: PointerPhase, dx: f32, dy: f32) {
    sim.input(InputEvent::Pointer {
        id: 9,
        phase,
        x: 400.0 + dx,
        y: 250.0 + dy,
        dx,
        dy,
        buttons: if matches!(phase, PointerPhase::Down | PointerPhase::Move) {
            1
        } else {
            0
        },
        at_ms: 0.0,
        player: 0,
    });
}

fn swipe(sim: &mut Sim<ExactBeatEmUp>, dx: f32, dy: f32) {
    pointer(sim, PointerPhase::Down, 0.0, 0.0);
    pointer(sim, PointerPhase::Move, dx, dy);
    pointer(sim, PointerPhase::Up, 0.0, 0.0);
    sim.run(8.334);
}

#[test]
fn starts_close_and_low_and_swiping_changes_movement_frame() {
    let mut sim = start();
    let player = sim.global_position("player").unwrap();
    let offset = sim.local_position("camera").unwrap() - player;
    assert!((0.8..1.5).contains(&offset.y), "{offset}");
    assert!((4.0..5.0).contains(&offset.z), "{offset}");
    swipe(&mut sim, std::f32::consts::FRAC_PI_2 / 0.004, 0.0);
    assert!(sim.local_position("camera").unwrap().x < player.x - 4.0);
    let before = sim.global_position("player").unwrap();
    sim.hold("KeyW", 250.0);
    let walked = sim.global_position("player").unwrap() - before;
    assert!(walked.x > 0.5 && walked.z.abs() < 0.01, "{walked}");
}

#[test]
fn hover_and_action_controls_do_not_orbit_but_a_second_finger_can() {
    let mut sim = start();
    sim.input(InputEvent::Pointer {
        id: 9,
        phase: PointerPhase::Move,
        x: 100.0,
        y: 100.0,
        dx: 90.0,
        dy: 20.0,
        buttons: 0,
        at_ms: 0.0,
        player: 0,
    });
    sim.run(8.334);
    assert_eq!(sim.world().resource::<Orbit>().yaw, 0.0);
    for (phase, x) in [(PointerPhase::Down, 40.0), (PointerPhase::Move, 80.0)] {
        sim.input(InputEvent::Control {
            name: "move".into(),
            id: 1,
            phase,
            x,
            y: 40.0,
            at_ms: 0.0,
            player: 0,
        });
    }
    sim.run(8.334);
    let following = sim.world().resource::<Orbit>().yaw;
    assert!(
        following.abs() < 0.04,
        "the stick only starts a smooth follow"
    );
    swipe(&mut sim, 160.0, 0.0);
    assert!((sim.world().resource::<Orbit>().yaw - following + 0.64).abs() < 0.001);
    let yaw = sim.world().resource::<Orbit>().yaw;
    pointer(&mut sim, PointerPhase::Down, 0.0, 0.0);
    pointer(&mut sim, PointerPhase::Move, 200.0, 0.0);
    pointer(&mut sim, PointerPhase::Cancel, 0.0, 0.0);
    sim.run(8.334);
    assert_eq!(sim.world().resource::<Orbit>().yaw, yaw);
}

#[test]
fn tilt_is_bounded_and_the_lens_stays_in_front_of_walls() {
    let mut sim = start();
    swipe(&mut sim, 0.0, 10_000.0);
    assert!(sim.world().resource::<Orbit>().elevation <= 0.95);
    swipe(&mut sim, 0.0, -20_000.0);
    assert!(sim.world().resource::<Orbit>().elevation >= -0.12);
    assert!(sim.get::<Transform>("camera").unwrap().rotation.is_finite());

    let mut sim = start();
    sim.world_mut().spawn_named(
        "camera-wall",
        (
            Transform::at(0.0, 2.0, 2.0),
            Collider {
                shape: Shape::Box {
                    half: Vec3::new(4.0, 2.0, 0.2),
                },
                layer: park::SOLID,
                ..Collider::default()
            },
        ),
    );
    sim.run(25.0);
    let eye = sim.local_position("camera").unwrap();
    assert!(eye.z > 0.5 && eye.z < 1.6, "{eye}");
    assert!(physics::overlap(
        sim.world(),
        &Shape::Sphere { radius: 0.21 },
        Transform::at(eye.x, eye.y, eye.z),
        park::SOLID
    )
    .is_empty());
}

#[test]
fn shake_is_subtle_and_a_save_resumes_the_same_camera() {
    let mut sim = start();
    let before = sim.local_position("camera").unwrap();
    sim.run(120.0);
    let moved = sim.local_position("camera").unwrap().distance(before);
    assert!(moved > 0.0001 && moved < 0.02, "idle sway: {moved}");
    swipe(&mut sim, 180.0, 45.0);
    sim.hold("KeyW", 150.0);
    let saved = sim.save().unwrap();
    let mut restored = start();
    restored.restore(&saved).unwrap();
    for game in [&mut sim, &mut restored] {
        game.hold("KeyD", 200.0);
    }
    assert_eq!(
        sim.local_position("camera"),
        restored.local_position("camera")
    );
    assert_eq!(sim.world().hash(), restored.world().hash());
}

#[test]
fn camera_follows_every_travel_direction_without_bending_a_held_input() {
    for (key, direction) in [
        ("KeyW", -Vec3::Z),
        ("KeyS", Vec3::Z),
        ("KeyA", -Vec3::X),
        ("KeyD", Vec3::X),
    ] {
        let mut sim = start();
        let before = sim.global_position("player").unwrap();
        sim.key_down(key);
        let mut previous = sim.world().resource::<Orbit>().yaw;
        for _ in 0..168 {
            sim.run(8.334);
            let yaw = sim.world().resource::<Orbit>().yaw;
            assert!(
                math::wrap_angle(yaw - previous).abs() < 0.033,
                "{key}: camera snapped"
            );
            previous = yaw;
        }
        let walked = sim.global_position("player").unwrap() - before;
        let planar = walked * Vec3::new(1.0, 0.0, 1.0);
        assert!(planar.dot(direction) > 3.0, "{key}: {planar}");
        assert!(
            planar.cross(direction).length() < 0.01,
            "{key}: curved path {planar}"
        );
        let forward = Quat::from_rotation_y(previous) * -Vec3::Z;
        assert!(
            forward.dot(direction) > 0.99,
            "{key}: camera forward {forward}"
        );
        sim.key_up(key);
        sim.run(500.0);
        assert_eq!(
            sim.world().resource::<Orbit>().yaw,
            previous,
            "idle view stays put"
        );
    }
}

#[test]
fn manual_look_has_priority_then_follow_resumes_while_moving() {
    let mut sim = start();
    sim.key_down("KeyD");
    sim.run(450.0);
    pointer(&mut sim, PointerPhase::Down, 0.0, 0.0);
    pointer(&mut sim, PointerPhase::Move, 120.0, 0.0);
    sim.run(8.334);
    let manual = sim.world().resource::<Orbit>().yaw;
    sim.run(1000.0);
    assert_eq!(
        sim.world().resource::<Orbit>().yaw,
        manual,
        "a held finger owns the view"
    );
    pointer(&mut sim, PointerPhase::Up, 0.0, 0.0);
    sim.run(500.0);
    assert_eq!(
        sim.world().resource::<Orbit>().yaw,
        manual,
        "release has a grace period"
    );
    let from = sim.global_position("player").unwrap();
    sim.run(1400.0);
    let travel = (sim.global_position("player").unwrap() - from) * Vec3::new(1.0, 0.0, 1.0);
    let forward = Quat::from_rotation_y(sim.world().resource::<Orbit>().yaw) * -Vec3::Z;
    assert!(
        forward.dot(travel.normalize()) > 0.99,
        "follow resumed behind travel"
    );
}
