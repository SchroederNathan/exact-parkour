use exact_beat_em_up_logic::stickman;
use exact_game::Vec3;

#[test]
fn mannequin_is_a_closed_smooth_mesh_with_valid_skin_weights() {
    let rig = stickman::rig(1.8);
    let model = stickman::model(&rig);
    model.validate().unwrap();
    let mesh = &model.meshes[0];
    // One draw call, with room to run on phones alongside the level geometry.
    assert_eq!(model.meshes.len(), 1);
    assert!(mesh.positions.len() / 3 < 15_000);
    let at = |i: u32| Vec3::from_slice(&mesh.positions[i as usize * 3..]);
    let normal = |i: u32| Vec3::from_slice(&mesh.normals[i as usize * 3..]);
    let mut edges = std::collections::BTreeMap::<(u32, u32), (u32, i32)>::new();
    for tri in mesh.indices.chunks_exact(3) {
        let face = (at(tri[1]) - at(tri[0])).cross(at(tri[2]) - at(tri[0]));
        assert!(face.length_squared() > 1e-16, "degenerate face {tri:?}");
        assert!(face.dot(normal(tri[0]) + normal(tri[1]) + normal(tri[2])) > 0.0);
        for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let edge = edges.entry((a.min(b), a.max(b))).or_default();
            edge.0 += 1;
            edge.1 += if a < b { 1 } else { -1 };
        }
    }
    assert!(
        edges.values().all(|&edge| edge == (2, 0)),
        "open or reversed shell"
    );
    for i in 0..mesh.positions.len() / 3 {
        assert!((normal(i as u32).length() - 1.0).abs() < 1e-5);
        let weights = &mesh.weights[i * 4..i * 4 + 4];
        assert!((weights.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        for (&joint, &weight) in mesh.joints[i * 4..i * 4 + 4].iter().zip(weights) {
            assert!(weight >= 0.0 && joint < rig.bones().len() as u16);
        }
    }
    assert!(model.bounds[1] >= -0.005 && (model.bounds[4] - 1.8).abs() < 1e-5);
}

#[test]
fn mannequin_scales_with_the_rig_and_preserves_animation_and_sockets() {
    let rig = stickman::rig(1.8);
    let model = stickman::model(&rig);
    let large = stickman::model(&stickman::rig(2.7));
    for (a, b) in model.meshes[0]
        .positions
        .iter()
        .zip(&large.meshes[0].positions)
    {
        assert!((a * 1.5 - b).abs() < 1e-5);
    }
    let skeleton = rig.model([]);
    assert_eq!(
        model.skins[0].inverse_binds,
        skeleton.skins[0].inverse_binds
    );
    assert_eq!(model.skins[0].joints, skeleton.skins[0].joints);
    for (actual, expected) in model.nodes.iter().zip(&skeleton.nodes) {
        assert_eq!(actual.name, expected.name);
        assert_eq!(actual.parent, expected.parent);
        assert_eq!(actual.transform, expected.transform);
    }
    assert!(model.nodes.iter().any(|n| n.name == "hand_l"));
    assert!(model.nodes.iter().any(|n| n.name == "hand_r"));
    assert!(model.clips.len() > 20);
}
