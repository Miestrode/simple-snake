use bevy::prelude::*;

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: String::from("Snake!"),
            width: 1000.0,
            height: 1000.0,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_event::<snake::GrowthEvent>()
        .add_event::<snake::FoodEvent>()
        .add_event::<snake::GameOverEvent>()
        .add_plugin(snake::SnakeActionPlugin)
        .add_plugins(DefaultPlugins)
        .run();
}
