use bevy::{
    prelude::*,
    render::pass::ClearColor
};
use hsluv::*;

use bevy_rapier2d::{na::{Isometry2, Vector2}, physics::{
    EventQueue, RapierConfiguration, RapierPhysicsPlugin, RigidBodyHandleComponent}, 
    rapier::{
        dynamics::RigidBodySet,  dynamics::JointSet, 
        geometry::{ColliderSet, ContactEvent}}};
use bevy_rapier2d::rapier::dynamics::RigidBodyBuilder;
use bevy_rapier2d::rapier::geometry::ColliderBuilder;

const BAT_WIDTH:f32 = 40.0;
const BAT_HEIGHT:f32 = 5.0;
const BALL_RADIUS:f32 = 5.0;

const RAPIER_SCALE:f32 = 20.0;

const LEVEL_WIDTH:f32 = 800.0;
const LEVEL_HEIGHT:f32 = 600.0;

const BALL_TEXTURE : &str = "png/ball.png";
const BAT_TEXTURE : &str = "png/bat.png";
const BRICK_TEXTURE : &str = "png/brick.png";

const FONT : &str = "fonts/FiraSans-Bold.ttf";

fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "Bevy/Rapier2d despawn example".to_string(),
            width: LEVEL_WIDTH,
            height:LEVEL_HEIGHT,
            vsync: true,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin)

        .add_resource(Score { score: 0 })
        .add_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
  
        .add_startup_system(setup_rapier.system())
        .add_startup_system(setup_ui.system())

        // Setup the level
        .add_startup_system(setup_walls.system())
        .add_startup_system(spawn_bricks.system())
        .add_startup_system(spawn_ball.system())
        .add_startup_system(spawn_bat.system())

        .add_system(player_movement_rapier_system.system())
        .add_system(scoring_system.system())

        // Add the collision checks
        .add_system_to_stage(stage::POST_UPDATE, check_collision_events.system())
        
        .run();
}

fn setup_rapier( mut rapier_config: ResMut<RapierConfiguration>) {
    rapier_config.gravity = Vector2::zeros();
    // While we want our sprite to look ~40 px square, we want to keep the physics units smaller
    // to prevent float rounding problems. To do this, we set the scale factor in RapierConfiguration
    // and divide our sprite_size by the scale.
     rapier_config.scale = RAPIER_SCALE;

     // Use frame rate indipendent physics
     rapier_config.time_dependent_number_of_timesteps = true;
}

fn setup_ui(commands: &mut Commands, asset_server: Res<AssetServer>){
    commands
        // cameras
        .spawn(Camera2dBundle::default())
        .spawn(CameraUiBundle::default())

        .spawn(TextBundle {
            text: Text {
                font: asset_server.load(FONT),
                value: "".to_string(),
                style: TextStyle {
                    color: Color::rgb(1.0, 0.5, 1.0),
                    font_size: 20.0,
                    ..Default::default()
                },
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(5.0),
                    left: Val::Px(LEVEL_WIDTH/2.0-50.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        });
}

enum PhysicsEntityType {
    Wall,
    Ball,
    Brick,
    Bat
}

struct Score {
    score: u32,
}

struct Bat {
    speed: f32,
}

struct Ball {
}

struct Brick {
}

fn spawn_ball(commands: &mut Commands,
     mut materials: ResMut<Assets<ColorMaterial>>, 
    asset_server: Res<AssetServer>) {

    let ball_texture_handle = asset_server.load(BALL_TEXTURE);
 
    let entity =  commands
        .spawn(SpriteBundle {
            material: materials.add(ball_texture_handle.into()),
            transform: Transform::from_translation(Vec3::new(0.0, -50.0, 1.0)),
            sprite: Sprite::new(Vec2::new(BALL_RADIUS*2.0, BALL_RADIUS*2.0)),
            ..Default::default()
        })
        .with(RigidBodyBuilder::new_dynamic()
            .position(Isometry2::new(Vector2::new(0.0,-50.0/RAPIER_SCALE),0.0))
            .linvel(3.0, -12.0) // some random starting velocity
        )
        .with(Ball {}).current_entity().unwrap();

      
    let collider = ColliderBuilder::ball( BALL_RADIUS/RAPIER_SCALE)
            .restitution(1.0)   // Bouncy
            .friction(0.0)      // No friction
            // Set the user data: first 64 bits is the entity type and the second 64 bits is the entity ID
            .user_data((entity.to_bits() as u128)<<64 | PhysicsEntityType::Ball as u128);

    commands.insert_one(entity, collider);
}

fn spawn_bat(commands: &mut Commands, mut materials: ResMut<Assets<ColorMaterial>>, asset_server: Res<AssetServer>) {
    let bat_texture_handle = asset_server.load(BAT_TEXTURE);
     commands
        .spawn(SpriteBundle {
            material: materials.add(bat_texture_handle.into()),
            transform: Transform::from_translation(Vec3::new(0.0, -215.0, 0.0)),
            sprite: Sprite::new(Vec2::new(BAT_WIDTH, BAT_HEIGHT)),
            ..Default::default()
        })
        .with(RigidBodyBuilder::new_kinematic()
            .position(Isometry2::new(Vector2::new(0.0,-215.0/RAPIER_SCALE),0.0))
        )
        .with(ColliderBuilder::round_cuboid(
            (BAT_WIDTH/RAPIER_SCALE) / 2.0,
            (BAT_HEIGHT/RAPIER_SCALE) / 2.0,
            (5.0/RAPIER_SCALE) / 2.0
        ).restitution(2.0).friction(1.0).user_data(PhysicsEntityType::Bat as u128))
        .with(Bat { speed: 20.0 });    
}

fn spawn_wall(commands: &mut Commands, offset: Vec2, size: Vec2,  wall_material: &Handle<ColorMaterial>) {

    commands
        .spawn(SpriteBundle {
            material: wall_material.clone(),
            transform: Transform::from_translation(Vec3::new(offset.x , offset.y, 0.0)),
            sprite: Sprite::new(size),
            ..Default::default()
        })
        .with(RigidBodyBuilder::new_static()
            .position(Isometry2::new(Vector2::new(offset.x/RAPIER_SCALE,offset.y/RAPIER_SCALE),0.0))
        )
        .with(ColliderBuilder::cuboid(
            (size.x/RAPIER_SCALE) / 2.0,
            (size.y/RAPIER_SCALE) / 2.0,
        )
        .restitution(1.0)   // Bouncy
        .friction(0.0)      // No friction
        // Set the user data: first 64 bits is the entity type and the second 64 bits is the entity ID
        // In this case we dont care about the entity ID
        .user_data(PhysicsEntityType::Wall as u128)
    );
}

fn setup_walls(commands: &mut Commands, mut materials: ResMut<Assets<ColorMaterial>>, _asset_server: Res<AssetServer>) {

    // Add walls
    let wall_material = materials.add(Color::rgb(0.8, 0.8, 0.8).into());
    let wall_thickness = 25.0;
    let bounds = Vec2::new(LEVEL_WIDTH, LEVEL_HEIGHT);
  
    spawn_wall(commands, Vec2::new(-bounds.x / 2.0,0.0), Vec2::new(wall_thickness, bounds.y + wall_thickness), &wall_material);
    spawn_wall(commands, Vec2::new(bounds.x / 2.0,0.0), Vec2::new(wall_thickness, bounds.y + wall_thickness), &wall_material);
    spawn_wall(commands, Vec2::new(0.0,-bounds.y / 2.0), Vec2::new(bounds.x + wall_thickness, wall_thickness), &wall_material);
    spawn_wall(commands, Vec2::new(0.0,bounds.y / 2.0), Vec2::new(bounds.x + wall_thickness, wall_thickness), &wall_material);
  
}

fn spawn_bricks(commands: &mut Commands, mut materials: ResMut<Assets<ColorMaterial>>, asset_server: Res<AssetServer>) {

    let brick_texture_handle = asset_server.load(BRICK_TEXTURE);
  
    // Add bricks
    let brick_rows = 14;
    let brick_columns = 22;
    let brick_spacing = 1.0;
    let brick_size = Vec2::new(30.0, 10.0);
    let bricks_width = brick_columns as f32 * (brick_size.x + brick_spacing) - brick_spacing;
    // center the bricks and move them up a bit
    let bricks_offset = Vec3::new(-(bricks_width - brick_size.x) / 2.0, 100.0, 0.0);

    for row in 0..brick_rows {
        let y_position = row as f32 * (brick_size.y + brick_spacing);
        let rgb = hsluv_to_rgb(((row as f64)/(brick_rows as f64)*360.0, 60.0, 40.0));
    
        let def_brick_mat = materials.add(brick_texture_handle.clone().into());
        let brick_material = def_brick_mat.clone();

        materials.get_mut(&brick_material).unwrap().color.set_r(rgb.0 as f32);
        materials.get_mut(&brick_material).unwrap().color.set_g(rgb.1 as f32);
        materials.get_mut(&brick_material).unwrap().color.set_b(rgb.2 as f32);

        for column in 0..brick_columns {

            let brick_position = Vec3::new(
                column as f32 * (brick_size.x + brick_spacing),
                y_position,
                0.0,
            ) + bricks_offset;

            // Track the entity so we can use it when we create the collider
            let entity = commands
                // brick
                .spawn(SpriteBundle {
                    material: brick_material.clone(),
                    sprite: Sprite::new(brick_size),
                    transform: Transform::from_translation(brick_position),
                    ..Default::default()
                })
                .with(Brick{})
                .with( RigidBodyBuilder::new_static()
                    .position(Isometry2::new(Vector2::new(brick_position.x/RAPIER_SCALE,brick_position.y/RAPIER_SCALE),0.0))
                )
                .current_entity().unwrap();
                

            let collider = ColliderBuilder::cuboid(
                    (brick_size.x/RAPIER_SCALE) / 2.0,
                    (brick_size.y/RAPIER_SCALE) / 2.0,
                ).restitution(1.0)

                // Set the user data: first 64 bits is the entity type and the second 64 bits is the entity ID
                .user_data((entity.to_bits() as u128)<<64 | PhysicsEntityType::Brick as u128);

            commands.insert_one(entity, collider);
        }
    }
}

fn check_collision_events(
    commands: &mut Commands,
    events: Res<EventQueue>,
    mut bodies:ResMut<RigidBodySet>,
    mut colliders:ResMut<ColliderSet>,
    mut score:ResMut<Score>
) {
    while let Ok(contact_event) = events.contact_events.pop() {
        match contact_event{
            ContactEvent::Started(c0,c1)=>{
                let collider0 = colliders.get(c0).unwrap(); 
                let collider1 = colliders.get(c1).unwrap(); 
                
                // Get the user data: first 64 bits is the entity type and the second 64 bits is the entity ID
                let t0bits =  (collider0.user_data & 0xFFFFFFFFFFFFFFFF) as u64 ;
                let t1bits =  (collider1.user_data & 0xFFFFFFFFFFFFFFFF) as u64 ;

                if t0bits !=  PhysicsEntityType::Wall as u64 && t1bits != PhysicsEntityType::Wall as u64 {

                    let e0 = Entity::from_bits((collider0.user_data>>64) as u64);
                    let e1 = Entity::from_bits((collider1.user_data>>64) as u64);

                    if t0bits == PhysicsEntityType::Brick as u64{
                        commands.despawn(e0);
                        bodies.remove(collider0.parent(), &mut colliders, &mut JointSet::new());
                         
                        // Do something fun like add to the score or whatever
                        score.score += 10;
                    }
                    else if t1bits == PhysicsEntityType::Brick as u64{
                        commands.despawn(e1);
                        bodies.remove(collider1.parent(), &mut colliders, &mut JointSet::new());

                        // Do something fun like add to the score or whatever
                        score.score += 10;
                    }     
                }
            }
            ContactEvent::Stopped(_, _) => {}
        }
    }
}

fn player_movement_rapier_system(
    keyboard_input: Res<Input<KeyCode>>,
    rapier_parameters: Res<RapierConfiguration>,
    mut rigid_bodies: ResMut<RigidBodySet>,
    player_info: Query<(&Bat, &RigidBodyHandleComponent)>,
) {
    for (player, rigid_body_component) in player_info.iter() {
        let mut direction = 0.0;
        if keyboard_input.pressed(KeyCode::Left) || keyboard_input.pressed(KeyCode::A) {
            direction -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::Right) || keyboard_input.pressed(KeyCode::D) {
            direction += 1.0;
        }

        let mut move_delta = Vector2::new(direction, 0.0);
        if move_delta != Vector2::zeros() {
            // Note that the RapierConfiguration::Scale factor is also used here to transform
            // the move_delta from: 'pixels/second' to 'physics_units/second'
            move_delta /= move_delta.magnitude() * rapier_parameters.scale;
        }

        let width = ((LEVEL_WIDTH- BAT_WIDTH) / rapier_parameters.scale) / 2.0 ;
        if let Some(rb) = rigid_bodies.get_mut(rigid_body_component.handle()) {
            let pos = rb.position().translation.vector;
            let mut new_pos = pos  + move_delta*player.speed;
            if new_pos.x < -width {
                new_pos.x = -width;
            }
            if new_pos.x > width {
                new_pos.x = width;
            }

            rb.set_next_kinematic_position(Isometry2::new(new_pos, 0.0));
        }
    }
}

fn scoring_system(scoreboard: Res<Score>, mut query: Query<&mut Text>) {
    for mut text in query.iter_mut() {
        text.value = format!("{}", scoreboard.score);
    }
}
