use std::ops::Sub;

use bevy::{core::FixedTimestep, prelude::*};
use rand::prelude::*;

struct Arena(u32, u32);

struct Size(u32, u32);

struct Materials {
    snake: Handle<ColorMaterial>,
    food: Handle<ColorMaterial>,
}

#[derive(PartialEq, Clone, Copy)]
enum SnakeState {
    Left,
    Right,
    Down,
    Up,
}

#[derive(Clone, Copy)]
struct LatestState(SnakeState);

impl LatestState {
    fn get_opposite(snake_state: SnakeState) -> SnakeState {
        match snake_state {
            SnakeState::Left => SnakeState::Right,
            SnakeState::Right => SnakeState::Left,
            SnakeState::Down => SnakeState::Up,
            SnakeState::Up => SnakeState::Down,
        }
    }

    fn switch(&mut self, new_state: SnakeState, compared_to: SnakeState) {
        if Self::get_opposite(compared_to) != new_state {
            self.0 = new_state;
        }
    }
}

struct SnakeHead(SnakeState);

struct SnakeSegment;

struct SnakeSegments(Vec<Entity>);

#[derive(PartialEq, Clone, Copy)]
struct Position(i32, i32);

impl Sub for Position {
    type Output = Position;

    fn sub(self, rhs: Self) -> Self::Output {
        Position(self.0 - rhs.0, self.1 - rhs.1)
    }
}

struct Food;

pub struct GrowthEvent;
pub struct FoodEvent;
pub struct GameOverEvent;

struct LastTailPosition(Option<Position>);

fn update_latest_state(
    input: Res<Input<KeyCode>>,
    mut latest_state: ResMut<LatestState>,
    head: Query<&SnakeHead>,
) {
    let head_state = head.iter().next().unwrap().0;

    if input.pressed(KeyCode::Left) {
        latest_state.switch(SnakeState::Left, head_state);
    }

    if input.pressed(KeyCode::Right) {
        latest_state.switch(SnakeState::Right, head_state);
    }

    if input.pressed(KeyCode::Up) {
        latest_state.switch(SnakeState::Up, head_state);
    }

    if input.pressed(KeyCode::Down) {
        latest_state.switch(SnakeState::Down, head_state);
    }
}

fn move_snake(
    arena: Res<Arena>,
    mut game_over_writer: EventWriter<GameOverEvent>,
    mut last_position: ResMut<LastTailPosition>,
    segments: ResMut<SnakeSegments>,
    latest_state: Res<LatestState>,
    mut heads: Query<(Entity, &mut SnakeHead)>,
    mut positions: Query<&mut Position>,
) {
    for (entity, mut head) in heads.iter_mut() {
        let segment_positions = segments
            .0
            .iter()
            .map(|&entity| *positions.get_mut(entity).unwrap())
            .collect::<Vec<Position>>();

        // This part must be done before the head is moved, otherwise two segments would be in the same spot.
        last_position.0 = Some(*segment_positions.last().unwrap());

        let mut head_position = positions.get_mut(entity).unwrap();
        head.0 = latest_state.0;

        match head.0 {
            SnakeState::Left => head_position.0 -= 1,
            SnakeState::Right => head_position.0 += 1,
            SnakeState::Down => head_position.1 -= 1,
            SnakeState::Up => head_position.1 += 1,
        }

        if segment_positions.contains(&head_position)
            || head_position.0 < 0
            || head_position.1 < 0
            || head_position.0 as u32 >= arena.0
            || head_position.1 as u32 >= arena.1
        {
            game_over_writer.send(GameOverEvent);
        }

        segment_positions
            .iter()
            // Skips the head since we already moved it. This way every segment moves to the position of the one in front of it.
            .zip(segments.0.iter().skip(1))
            .for_each(|(&position, &segment)| *positions.get_mut(segment).unwrap() = position);
    }
}

fn update_transform_position(
    windows: Res<Windows>,
    arena: Res<Arena>,
    mut positions: Query<(&Position, &mut Transform)>,
) {
    fn update_axis(position: f32, length: f32, subdivisions: f32) -> f32 {
        let tile_size = length / subdivisions;

        position * tile_size - length / 2.0 + tile_size / 2.0
    }

    let window = windows.get_primary().unwrap();

    for (position, mut transform) in positions.iter_mut() {
        transform.translation.x = update_axis(position.0 as f32, window.width(), arena.0 as f32);
        transform.translation.y = update_axis(position.1 as f32, window.height(), arena.1 as f32);
    }
}

fn update_size(windows: Res<Windows>, arena: Res<Arena>, mut sprites: Query<(&Size, &mut Sprite)>) {
    let window = windows.get_primary().unwrap();

    for (size, mut sprite) in sprites.iter_mut() {
        sprite.size = Vec2::new(
            window.width() / arena.0 as f32 * size.0 as f32,
            window.height() / arena.1 as f32 * size.1 as f32,
        );
    }
}

fn snake_eat(
    mut growth_writer: EventWriter<GrowthEvent>,
    mut food_writer: EventWriter<FoodEvent>,
    heads: Query<&Position, With<SnakeHead>>,
    food: Query<(Entity, &Position), With<Food>>,
    mut commands: Commands,
) {
    for head_position in heads.iter() {
        for (food, food_position) in food.iter() {
            if head_position == food_position {
                commands.entity(food).despawn();
                growth_writer.send(GrowthEvent);
                food_writer.send(FoodEvent);
            }
        }
    }
}

fn spawn_segment(
    material: Handle<ColorMaterial>,
    mut commands: Commands,
    position: Position,
) -> Entity {
    commands
        .spawn_bundle(SpriteBundle {
            material,
            ..Default::default()
        })
        .insert(position)
        .insert(Size(1, 1))
        .insert(SnakeSegment)
        .id()
}

fn grow_snake(
    mut segments: ResMut<SnakeSegments>,
    materials: Res<Materials>,
    last_position: ResMut<LastTailPosition>,
    commands: Commands,
    mut growth_reader: EventReader<GrowthEvent>,
) {
    if growth_reader.iter().next().is_some() {
        segments.0.push(spawn_segment(
            materials.snake.clone(),
            commands,
            last_position.0.unwrap(),
        ))
    }
}

fn game_over(
    food_writer: EventWriter<FoodEvent>,
    arena: Res<Arena>,
    segments: ResMut<SnakeSegments>,
    materials: Res<Materials>,
    mut commands: Commands,
    mut reader: EventReader<GameOverEvent>,
    entities: Query<Entity, With<Position>>,
    mut latest_state: ResMut<LatestState>,
) {
    if reader.iter().next().is_some() {
        for entity in entities.iter() {
            commands.entity(entity).despawn();
        }

        latest_state.0 = SnakeState::Right;
        spawn_snake(food_writer, arena, segments, materials, commands);
    }
}

fn spawn_snake(
    mut food_writer: EventWriter<FoodEvent>,
    arena: Res<Arena>,
    mut segments: ResMut<SnakeSegments>,
    materials: Res<Materials>,
    mut commands: Commands,
) {
    let center = Position((arena.0 / 2) as i32, (arena.1 / 2) as i32);

    segments.0 = vec![
        commands
            .spawn_bundle(SpriteBundle {
                material: materials.snake.clone(),
                ..Default::default()
            })
            .insert(center)
            .insert(Size(1, 1))
            .insert(SnakeHead(SnakeState::Right))
            .id(),
        spawn_segment(materials.snake.clone(), commands, center - Position(1, 0)),
    ];

    food_writer.send(FoodEvent);
}

fn generate_random_position(arena: Res<Arena>, taken: Vec<Position>) -> Position {
    let mut rng = rand::thread_rng();
    let mut all_positions = Vec::with_capacity(arena.0 as usize * arena.1 as usize);

    for x in 0..arena.0 {
        for y in 0..arena.1 {
            all_positions.push(Position(x as i32, y as i32))
        }
    }

    all_positions = all_positions
        .iter()
        .copied()
        .filter(|&position| !taken.contains(&position))
        .collect();
    *all_positions.choose(&mut rng).unwrap()
}

fn spawn_food(
    mut food_reader: EventReader<FoodEvent>,
    arena: Res<Arena>,
    materials: Res<Materials>,
    positions: Query<&Position>,
    mut commands: Commands,
) {
    if food_reader.iter().next().is_some() {
        commands
            .spawn_bundle(SpriteBundle {
                material: materials.food.clone(),
                ..Default::default()
            })
            .insert(generate_random_position(
                arena,
                positions.iter().copied().collect(),
            ))
            .insert(Size(1, 1))
            .insert(Food);
    }
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands.insert_resource(Materials {
        snake: materials.add(ColorMaterial::color(Color::rgb(1.0, 0.0, 0.0))),
        food: materials.add(ColorMaterial::color(Color::rgb(1.0, 1.0, 0.0))),
    });
    commands.insert_resource(LatestState(SnakeState::Right));
    commands.insert_resource(Arena(15, 15));
}

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
enum SnakeAction {
    Eat,
    Move,
}

pub struct SnakeActionPlugin;

impl Plugin for SnakeActionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(SnakeSegments(Vec::new()))
            .insert_resource(LastTailPosition(None))
            .add_startup_system(setup.system())
            .add_startup_stage("game_setup", SystemStage::single(spawn_snake.system()))
            .add_system(update_latest_state.system())
            .add_system(
                move_snake
                    .system()
                    .label(SnakeAction::Move)
                    .with_run_criteria(FixedTimestep::step(0.15)),
            )
            .add_system(
                snake_eat
                    .system()
                    .label(SnakeAction::Eat)
                    .after(SnakeAction::Move),
            )
            .add_system(grow_snake.system().after(SnakeAction::Eat))
            .add_system(spawn_food.system().after(SnakeAction::Eat))
            .add_system(game_over.system())
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new()
                    .with_system(update_transform_position.system())
                    .with_system(update_size.system()),
            );
    }
}
