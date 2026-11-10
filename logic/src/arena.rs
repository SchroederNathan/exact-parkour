use crate::body;
use exact_game::*;

const ROUND_GAP: f32 = 1.1;
const RAISE_SECONDS: f32 = 0.35;
const FIRE_SECONDS: f32 = 0.16;
const ENEMY_SPEED: f32 = 2.1;
const CONTACT_DISTANCE: f32 = 1.05;
const CONTACT_SECONDS: f32 = 0.65;
const TRACER_SECONDS: f32 = 0.08;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Data)]
pub enum CombatPhase {
    #[default]
    Ready,
    Raising,
    Firing,
    RoundClear,
    Defeated,
}

#[derive(Clone, Debug, Default, Resource)]
pub struct CombatState {
    pub phase: CombatPhase,
    pub round: u32,
    pub remaining: u32,
    pub defeated: u32,
    pub player_hits: u8,
    pub raise: f32,
    pub fire_timer: f32,
    pub round_timer: f32,
    pub left_next: bool,
    pub serial: u32,
}

#[derive(Clone, Debug, Default, Component)]
pub struct Enemy {
    pub health: u8,
    pub contacts: u8,
    pub next_contact: u64,
}

#[derive(Clone, Copy, Debug, Default, Component)]
pub struct Tracer {
    pub spawned: u64,
}

#[derive(Clone, Copy, Debug, Default, Component)]
pub struct MuzzleFlash {
    pub until: u64,
}

#[derive(Clone, Copy, Debug, Default, Component)]
pub struct Pistol {
    pub left: bool,
}

pub const ROUND_COUNTS: &[u32] = &[4, 6, 8, 10];

pub fn register(w: &mut World) {
    w.register::<Enemy>();
    w.register::<Tracer>();
    w.register::<MuzzleFlash>();
    w.register::<Pistol>();
    w.register_resource::<CombatState>();
}

pub fn setup(w: &mut World, player: Entity) {
    w.insert_resource(CombatState::default());
    let pistol_material = Material::rgb(0.04, 0.05, 0.07).metallic(0.85).rough(0.18);
    for (name, left, x) in [("pistol-left", true, 0.28), ("pistol-right", false, -0.28)] {
        w.spawn_named(
            name,
            (
                Parent(player),
                Transform::at(x, 0.82, -0.32),
                Mesh::cuboid(Vec3::new(0.12, 0.14, 0.42)),
                pistol_material.clone(),
                Pistol { left },
            ),
        );
    }
    spawn_round(w, 1);
}

fn round_count(round: u32) -> u32 {
    ROUND_COUNTS
        .get(round.saturating_sub(1) as usize)
        .copied()
        .unwrap_or_else(|| {
            ROUND_COUNTS.last().copied().unwrap_or(4) + (round.saturating_sub(4) * 2)
        })
}

fn spawn_round(w: &mut World, round: u32) {
    let count = round_count(round);
    let radius = 10.5 + (round as f32 * 0.45).min(4.0);
    for index in 0..count {
        let angle = (index as f32 / count as f32) * std::f32::consts::TAU + round as f32 * 0.37;
        let at = Vec3::new(
            math::sin(angle) * radius,
            body::HALF,
            math::cos(angle) * radius,
        );
        let serial = w.resource::<CombatState>().serial + index;
        w.spawn_named(
            format!("enemy-{serial}"),
            (
                Transform::at(at.x, at.y, at.z),
                Mesh::capsule(0.3, body::HEIGHT * 0.92),
                Material::hex(if index % 2 == 0 { "#c4475a" } else { "#8c3fa3" }),
                Enemy {
                    health: 1,
                    ..Enemy::default()
                },
            ),
        );
    }
    let mut state = w.resource_mut::<CombatState>();
    state.round = round;
    state.remaining = count;
    state.defeated = 0;
    state.serial += count;
    state.phase = CombatPhase::Raising;
    state.raise = 0.0;
    state.fire_timer = 0.0;
    state.player_hits = 0;
}

pub fn tick(w: &mut World) {
    let dt = w.dt();
    let now = w.tick();
    cleanup_tracers(w, now);
    let phase = w.resource::<CombatState>().phase;
    if phase == CombatPhase::Defeated {
        publish(w);
        return;
    }
    if phase == CombatPhase::Ready {
        w.resource_mut::<CombatState>().phase = CombatPhase::Raising;
    }
    advance_enemies(w, dt, now);
    apply_contacts(w, now);
    advance_weapons(w, dt, now);
    publish(w);
}

fn advance_enemies(w: &mut World, dt: f32, now: u64) {
    let player = w.require::<Transform>("player").position;
    let entities: Vec<Entity> = w
        .query::<(&Transform, &Enemy)>()
        .iter()
        .map(|(e, _)| e)
        .collect();
    for entity in entities {
        let mut pose = w.require_mut::<Transform>(entity);
        let delta = Vec3::new(player.x - pose.position.x, 0.0, player.z - pose.position.z);
        let distance = delta.length();
        if distance > CONTACT_DISTANCE {
            let step = delta.normalize_or_zero() * ENEMY_SPEED * dt;
            pose.position += step;
            pose.rotation = pose
                .looking_at(Vec3::new(player.x, pose.position.y, player.z), Vec3::Y)
                .rotation;
        }
        let _ = (distance, now);
    }
}

fn apply_contacts(w: &mut World, now: u64) {
    let player = w.require::<Transform>("player").position;
    let hits: Vec<Entity> = w
        .query::<(&Transform, &Enemy)>()
        .iter()
        .filter(|(_, (pose, enemy))| {
            pose.position.distance(player) < CONTACT_DISTANCE && enemy.next_contact <= now
        })
        .map(|(e, _)| e)
        .collect();
    if hits.is_empty() {
        return;
    }
    let dt = w.dt();
    for enemy_entity in hits {
        let mut enemy = w.require_mut::<Enemy>(enemy_entity);
        enemy.contacts = enemy.contacts.saturating_add(1);
        enemy.next_contact = now + (CONTACT_SECONDS / dt) as u64;
    }
    let mut state = w.resource_mut::<CombatState>();
    state.player_hits = state.player_hits.saturating_add(1);
    if state.player_hits >= 3 {
        state.phase = CombatPhase::Defeated;
    }
}

fn advance_weapons(w: &mut World, dt: f32, now: u64) {
    let (phase, raise, fire_timer, round_timer) = {
        let state = w.resource::<CombatState>();
        (
            state.phase,
            state.raise,
            state.fire_timer,
            state.round_timer,
        )
    };
    match phase {
        CombatPhase::Raising => {
            let next = raise + dt;
            {
                let mut state = w.resource_mut::<CombatState>();
                state.raise = next;
                if next >= RAISE_SECONDS {
                    state.phase = CombatPhase::Firing;
                    state.fire_timer = FIRE_SECONDS;
                }
            }
            pose_pistols(w, (next / RAISE_SECONDS).clamp(0.0, 1.0));
        }
        CombatPhase::Firing => {
            pose_pistols(w, 1.0);
            let next = fire_timer - dt;
            if next <= 0.0 {
                fire(w, now);
                w.resource_mut::<CombatState>().fire_timer = FIRE_SECONDS;
            } else {
                w.resource_mut::<CombatState>().fire_timer = next;
            }
        }
        CombatPhase::RoundClear => {
            let next = round_timer - dt;
            if next <= 0.0 {
                let round = w.resource::<CombatState>().round + 1;
                spawn_round(w, round);
            } else {
                w.resource_mut::<CombatState>().round_timer = next;
            }
        }
        CombatPhase::Defeated | CombatPhase::Ready => {}
    }
}

fn pose_pistols(w: &mut World, raised: f32) {
    for (_, (pose, pistol)) in w.query::<(&mut Transform, &Pistol)>().iter() {
        let y = 0.62 + 0.26 * raised;
        let z = -0.12 - 0.20 * raised;
        pose.position = Vec3::new(if pistol.left { 0.28 } else { -0.28 }, y, z);
        pose.rotation = Quat::from_rotation_x(-0.18 * raised);
    }
}

fn nearest_enemy(w: &World, origin: Vec3) -> Option<(Entity, Vec3)> {
    w.query::<(&Transform, &Enemy)>()
        .iter()
        .map(|(entity, (pose, _))| (entity, pose.position))
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(origin)
                .total_cmp(&b.distance_squared(origin))
        })
}

fn fire(w: &mut World, now: u64) {
    let player = w.require::<Transform>("player").position + Vec3::Y * 0.9;
    let Some((target, target_position)) = nearest_enemy(w, player) else {
        w.resource_mut::<CombatState>().phase = CombatPhase::RoundClear;
        w.resource_mut::<CombatState>().round_timer = ROUND_GAP;
        return;
    };
    let left = w.resource::<CombatState>().left_next;
    let muzzle = player + Vec3::new(if left { 0.28 } else { -0.28 }, 0.1, -0.28);
    let distance = muzzle.distance(target_position).max(0.1);
    let dir = (target_position - muzzle).normalize_or_zero();
    w.spawn((
        Transform::at(
            (muzzle.x + target_position.x) * 0.5,
            (muzzle.y + target_position.y) * 0.5,
            (muzzle.z + target_position.z) * 0.5,
        )
        .looking_at(target_position, Vec3::Y),
        Mesh::cuboid(Vec3::new(0.045, 0.045, distance)),
        Material::glow(if left {
            [1.0, 0.76, 0.18]
        } else {
            [0.24, 0.82, 1.0]
        }),
        Tracer { spawned: now },
    ));
    w.spawn((
        Transform::at(
            muzzle.x + dir.x * 0.18,
            muzzle.y + dir.y * 0.18,
            muzzle.z + dir.z * 0.18,
        ),
        Mesh::sphere(0.11),
        Material::glow([1.0, 0.65, 0.12]),
        MuzzleFlash { until: now + 5 },
    ));
    let dead = {
        let mut enemy = w.require_mut::<Enemy>(target);
        enemy.health = enemy.health.saturating_sub(1);
        enemy.health == 0
    };
    if dead {
        w.despawn(target);
        let mut state = w.resource_mut::<CombatState>();
        state.remaining = state.remaining.saturating_sub(1);
        state.defeated += 1;
        if state.remaining == 0 {
            state.phase = CombatPhase::RoundClear;
            state.round_timer = ROUND_GAP;
        }
    }
    w.resource_mut::<CombatState>().left_next = !left;
}

fn cleanup_tracers(w: &mut World, now: u64) {
    let life = (TRACER_SECONDS / w.dt()) as u64;
    let old: Vec<Entity> = w
        .query::<&Tracer>()
        .iter()
        .filter(|(_, tracer)| now.saturating_sub(tracer.spawned) > life)
        .map(|(entity, _)| entity)
        .collect();
    for entity in old {
        w.despawn(entity);
    }
    let flashes: Vec<Entity> = w
        .query::<&MuzzleFlash>()
        .iter()
        .filter(|(_, flash)| flash.until < now)
        .map(|(entity, _)| entity)
        .collect();
    for entity in flashes {
        w.despawn(entity);
    }
}

fn publish(w: &mut World) {
    let state = w.resource::<CombatState>();
    let round = state.round;
    let enemies = state.remaining;
    let hits = state.player_hits as u32;
    let phase = format!("{:?}", state.phase);
    drop(state);
    let (move_x, move_y) = {
        let stick = w.resource::<crate::TouchStick>();
        (stick.x, stick.y)
    };
    w.publish_record(&crate::Hud {
        mode: "arena".into(),
        race: "Rooftop Holdout".into(),
        subtitle: "Dual pistols".into(),
        seconds: 0.0,
        finished: false,
        checkpoint: 0,
        checkpoints: 0,
        falls: 0,
        round,
        enemies,
        hits,
        phase,
        move_x,
        move_y,
    });
}
