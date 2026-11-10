use exact_beat_em_up_logic::race::{definition, ROUTES};

#[test]
fn registry_keeps_three_distinct_routes() {
    assert_eq!(ROUTES.len(), 3);
    assert_ne!(definition(0).title, definition(1).title);
    assert_ne!(definition(1).title, definition(2).title);
    assert!(ROUTES.iter().all(|route| route.blocks.len() >= 7));
}
