use crate::state::AppState;
use bevy::prelude::*;
use bevy::ui::Interaction;

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

#[derive(Resource)]
pub struct MenuData {
    button_entity: Entity,
}

pub fn setup(mut commands: Commands) {
    println!("list setup");
    let button_entity = commands
        .spawn((
            Node {
                // center button
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            children![(
                Button,
                Node {
                    width: Val::Px(150.),
                    height: Val::Px(65.),
                    // horizontally center child text
                    justify_content: JustifyContent::Center,
                    // vertically center child text
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(NORMAL_BUTTON),
                children![(
                    Text::new("detail"),
                    TextFont {
                        font_size: 33.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                )],
            )],
        ))
        .id();
    commands.insert_resource(MenuData { button_entity });
}

#[allow(clippy::type_complexity)]
pub fn update(
    mut next_state: ResMut<NextState<AppState>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    println!("list update");
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                next_state.set(AppState::Detail);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
}

pub fn cleanup(mut commands: Commands, menu_data: Res<MenuData>) {
    println!("list cleanup");
    commands.entity(menu_data.button_entity).despawn();
}
