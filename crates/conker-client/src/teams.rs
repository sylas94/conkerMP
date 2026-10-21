//! The scoreboard, every mode's: the Scoreboard action (Tab, a gamepad's Select) opens and closes
//! it over the match. Each player by name (its [`Pseudonym`]; a bot as BOT and its slot), this
//! client's marked `>`, with what the mode ranks: free for all (Deathmatch, Tank) the place, kills,
//! deaths and, untimed, the lives left; the team modes ([`MatchView::teams`]) each team with its
//! score (War: Colors' flags, Heist's bags, Beach's and Raptor's points, Total War's kills) over its
//! players; Race the race table's places, laps and best laps.
//!
//! In the team modes it is the team screen too, as the original's before a match: while it is open
//! the player stands (`buffer_input` sends no pad): the stick pushed left or right moves it to the
//! team before or after (conker-sim's [`Teams::Switch`], wrapping: it respawns there; the server
//! refuses a team with no room for a human), pushed up or down (W / S) or C-left / C-right move the
//! BALANCE (2 teams; the first human's: bots to team 0, or to team 1), A or B closes it. Elsewhere
//! the player plays on under it. `team=T` asks for team T once the player is in (the lobby's pick,
//! headless tests).

use crate::bindings::{Action, Controls, device_input};
use crate::{BUTTON_B, C_LEFT, C_RIGHT, Config, FollowCamera};
use bevy::prelude::*;
use conker_rules::game::places;
use conker_sim::movement::BUTTON_A;
use conker_sim::net::{Combat, JoinChannel, MatchView, Pseudonym, RacerView, Teams};
use lightyear::prelude::*;

/// The stick pushed past this (N64 units) moves the team, as the results' menu.
const PUSH: i8 = 30;

/// Whether the screen is open, `team=` asked for, the match's teams (0 free for all) and the last
/// frame's pad (a push or press acts on its first frame).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct Screen {
    pub open: bool,
    asked: bool,
    teams: u8,
    last: (u16, i8, bool),
}

impl Screen {
    /// Open in a team mode: the pad picks a team, so the player stands.
    pub fn picking(&self) -> bool {
        self.open && self.teams > 0
    }

    /// One frame of the pad (`buttons`, the stick's x, `toggle`: Tab or Select held), the player on
    /// team `team` of `teams` with the BALANCE at `balance`: what to ask the server.
    pub fn step(
        &mut self,
        (buttons, stick_x, toggle): (u16, i8, bool),
        team: u8,
        teams: u8,
        balance: i8,
    ) -> Option<Teams> {
        let (last, last_x, last_toggle) =
            std::mem::replace(&mut self.last, (buttons, stick_x, toggle));
        let pressed = |bit: u16| buttons & bit != 0 && last & bit == 0;
        let pushed = |x: i8| x.unsigned_abs() > PUSH as u8;
        if toggle && !last_toggle {
            self.open = !self.open;
            return None;
        }
        if !self.open || teams == 0 {
            return None;
        }
        if pressed(BUTTON_A) || pressed(BUTTON_B) {
            self.open = false;
            return None;
        }
        if pushed(stick_x) && !pushed(last_x) {
            let next = if stick_x > 0 {
                team + 1
            } else {
                team + teams - 1
            };
            return Some(Teams::Switch(next % teams));
        }
        match () {
            _ if teams == 2 && pressed(C_LEFT) => Some(Teams::Balance(balance.saturating_add(1))),
            _ if teams == 2 && pressed(C_RIGHT) => Some(Teams::Balance(balance.saturating_sub(1))),
            _ => None,
        }
    }
}

/// The teams' names as the original's rows show them.
fn names(m: &MatchView) -> &'static [&'static str] {
    match () {
        _ if m.heist => &["REDS", "BLUES", "GREENS", "YELLOWS"],
        _ if m.beach => &["FRENCHYS", "TEDIZ"],
        _ if m.raptor => &["UGAS", "RAPTORS"],
        _ => &["SQUIRRELS", "TEDIZ"],
    }
}

/// One player on the board: its [`Combat`] and its pseudonym (bots have none).
pub type Entry<'a> = (&'a Combat, Option<&'a str>);

/// The name column's width: the server's longest pseudonym.
const NAME: usize = 20;

/// The mode's name, over the board.
fn mode(m: &MatchView) -> &'static str {
    match () {
        _ if m.heist => "WAR: HEIST",
        _ if m.beach => "WAR: BEACH",
        _ if m.raptor => "WAR: RAPTOR",
        _ if m.colors => "WAR: COLORS",
        _ if m.total_war => "WAR: TOTAL WAR",
        _ if m.race => "RACE",
        _ if m.tank => "TANK",
        _ => "DEATHMATCH",
    }
}

/// A place as the HUD shows it: 1ST, 2ND, 3RD, 4TH…, 11TH-13TH.
fn ordinal(n: u8) -> String {
    let suffix = match (n % 10, n % 100 / 10) {
        (1, t) if t != 1 => "ST",
        (2, t) if t != 1 => "ND",
        (3, t) if t != 1 => "RD",
        _ => "TH",
    };
    format!("{n}{suffix}")
}

/// `vis` (60 a second) as m:ss.
fn clock(vis: i32) -> String {
    let s = vis.max(0) / 60;
    format!("{}:{:02}", s / 60, s % 60)
}

/// A player's mark (`>` on this client's own) and name (or BOT and its slot), cut to [`NAME`].
fn who(own: &Combat, (c, name): Entry) -> (char, String) {
    let mark = if c.slot == own.slot { '>' } else { ' ' };
    let name = name.map_or_else(
        || format!("BOT {}", c.slot + 1),
        |n| n.chars().take(NAME).collect(),
    );
    (mark, name)
}

/// The board's text (`players`: everyone, this client's own among them).
pub fn text(m: &MatchView, own: &Combat, players: &[Entry]) -> String {
    let mut out = String::from(mode(m));
    if m.time > 0 {
        out += &format!("  {}", clock(m.timer));
    }
    let limit = if m.teams > 0 {
        m.score_limit
    } else {
        m.kill_limit
    };
    if limit > 0 && !m.race && !m.total_war {
        out += &format!("  FIRST TO {limit}");
    }
    out.push('\n');
    if m.race {
        race(&mut out, own, players, &m.racing.racers, m.racing.laps);
    } else if m.teams > 0 {
        teams(&mut out, m, own, players);
    } else {
        free_for_all(&mut out, m, own, players);
    }
    out + match m.teams {
        0 => "Tab: close",
        _ => "stick: change team, A: close",
    }
}

/// Free for all: the places (conker-rules' [`places`], as the HUD's), kills, deaths and, untimed,
/// the lives left (OUT once gone).
fn free_for_all(out: &mut String, m: &MatchView, own: &Combat, players: &[Entry]) {
    let lives = m.time == 0;
    let slots = (players.iter())
        .map(|p| usize::from(p.0.slot) + 1)
        .max()
        .unwrap_or(0);
    let mut standings = vec![None; slots];
    for (c, _) in players {
        standings[usize::from(c.slot)] = Some(crate::hud::standing(c));
    }
    let place = places(&standings, m.kill_limit);
    let mut rows: Vec<_> = (players.iter())
        .map(|p| (place[usize::from(p.0.slot)], *p))
        .collect();
    rows.sort_by_key(|(place, p)| (*place, p.0.slot));
    let head = if lives { " LIVES" } else { "" };
    *out += &format!("     {:<NAME$} KILLS DEATHS{head}\n", "NAME");
    for (place, p) in rows {
        let (mark, name) = who(own, p);
        let (kills, deaths) = (p.0.kills, p.0.deaths);
        *out += &format!(
            "{mark}{:<4}{name:<NAME$} {kills:>5} {deaths:>6}",
            ordinal(place)
        );
        if lives {
            *out += &match p.0.out {
                0 => format!(" {:>5}", p.0.lives_left),
                _ => "   OUT".into(),
            };
        }
        out.push('\n');
    }
}

/// The team modes: each team (the stick's order), `>` on this client's, with its score, then its
/// players by kills; the BALANCE with 2.
fn teams(out: &mut String, m: &MatchView, own: &Combat, players: &[Entry]) {
    let members = |t: usize| players.iter().filter(move |p| usize::from(p.0.team) == t);
    let (label, score) = match () {
        _ if m.heist => ("BAGS", m.heist_scores.map(u32::from).to_vec()),
        _ if m.colors => ("FLAGS", m.scores.map(u32::from).to_vec()),
        _ if m.beach || m.raptor => ("POINTS", m.scores.map(u32::from).to_vec()),
        _ => (
            "KILLS",
            (0..usize::from(m.teams))
                .map(|t| members(t).map(|p| u32::from(p.0.kills)).sum())
                .collect(),
        ),
    };
    *out += &format!("     {:<NAME$} KILLS DEATHS\n", "NAME");
    for (t, name) in names(m).iter().enumerate().take(usize::from(m.teams)) {
        let mark = if usize::from(own.team) == t { '>' } else { ' ' };
        let points = score.get(t).copied().unwrap_or(0);
        *out += &format!("{mark}{name:<9} {points} {label}\n");
        let mut on: Vec<_> = members(t).collect();
        on.sort_by_key(|p| (std::cmp::Reverse(p.0.kills), p.0.slot));
        for p in on {
            let (mark, name) = who(own, *p);
            let (kills, deaths) = (p.0.kills, p.0.deaths);
            *out += &format!("{mark}    {name:<NAME$} {kills:>5} {deaths:>6}\n");
        }
    }
    if m.teams == 2 {
        *out += &format!("BALANCE {:+} (W/S)\n", m.balance);
    }
}

/// Race: the race table's places, each racer's lap of `laps` (DONE once over the line) and best lap
/// (humans only: the table keeps no bot's times).
fn race(out: &mut String, own: &Combat, players: &[Entry], racers: &[RacerView], laps: u8) {
    let mut rows: Vec<_> = (racers.iter())
        .filter_map(|r| Some((r, *players.iter().find(|p| p.0.slot == r.slot)?)))
        .collect();
    rows.sort_by_key(|(r, _)| (r.place, r.slot));
    *out += &format!("     {:<NAME$}   LAP     BEST\n", "NAME");
    for (r, p) in rows {
        let (mark, name) = who(own, p);
        let lap = match r.order {
            0 => format!("{}/{laps}", (r.lap + 1).clamp(1, laps.max(1).into())),
            _ => "DONE".into(),
        };
        let best = match r.best {
            b if b < 0 => "-".into(),
            b => format!("{}.{:02}", clock(b), b % 60 * 100 / 60),
        };
        let place = ordinal(r.place);
        *out += &format!("{mark}{place:<4}{name:<NAME$} {lap:>5} {best:>8}\n");
    }
}

pub fn build(app: &mut App, draw: bool) {
    app.init_resource::<Screen>();
    app.add_systems(Update, ask);
    if draw {
        #[cfg(not(target_family = "wasm"))]
        app.add_systems(Startup, |mut commands: Commands| {
            commands.spawn((
                TeamText,
                Text::new(""),
                TextFont {
                    font_size: bevy::text::FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                Visibility::Hidden,
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(20.0),
                    left: Val::Percent(25.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
            ));
        });
        app.add_systems(Update, show.after(ask));
    }
}

/// The screen's text node (native).
#[cfg(not(target_family = "wasm"))]
#[derive(Component)]
struct TeamText;

/// The local pad on the screen, and `team=`'s ask once the player is in.
fn ask(
    cfg: Res<Config>,
    controls: Controls,
    game: Query<&MatchView>,
    own: Query<&Combat, With<FollowCamera>>,
    mut screen: ResMut<Screen>,
    mut senders: Query<&mut MessageSender<Teams>>,
) {
    let (Ok(m), Ok(c)) = (game.single(), own.single()) else {
        return;
    };
    let mut asks = Vec::new();
    if let Some(t) = cfg.team.filter(|_| !screen.asked && m.teams > 0) {
        screen.asked = true;
        asks.push(Teams::Switch(t));
    }
    screen.teams = m.teams;
    if cfg.script.is_none() {
        let pad = device_input(&controls.bindings, &controls.devices(0.0, Vec2::ZERO)).input;
        // Up / down move the BALANCE as C-left / C-right (Q and E are Holster and Interact now).
        let balance = match pad.stick_y {
            y if y > PUSH => C_LEFT,
            y if y < -PUSH => C_RIGHT,
            _ => 0,
        };
        let toggle = controls.pressed(Action::Scoreboard);
        let step = (pad.buttons | balance, pad.stick_x, toggle);
        asks.extend(screen.step(step, c.team, m.teams, m.balance));
    }
    for ask in asks {
        info!("teams: {ask:?}");
        for mut sender in &mut senders {
            sender.send::<JoinChannel>(ask);
        }
    }
}

/// The open screen: a text node natively, the page's `#teams` element in the browser.
#[allow(clippy::type_complexity)]
fn show(
    screen: Res<Screen>,
    game: Query<&MatchView>,
    own: Query<&Combat, With<FollowCamera>>,
    roster: Query<(&Combat, Option<&Pseudonym>), Or<(With<Predicted>, With<Interpolated>)>>,
    #[cfg(not(target_family = "wasm"))] mut node: Single<
        (&mut Text, &mut Visibility),
        With<TeamText>,
    >,
    #[cfg(target_family = "wasm")] mut shown: Local<String>,
) {
    let text = match (game.single(), own.single()) {
        (Ok(m), Ok(c)) if screen.open => {
            let players: Vec<Entry> = (roster.iter())
                .map(|(p, name)| (p, name.map(|n| n.0.as_str())))
                .collect();
            text(m, c, &players)
        }
        _ => String::new(),
    };
    #[cfg(not(target_family = "wasm"))]
    {
        let (text_node, visibility) = &mut *node;
        visibility.set_if_neq(match text.is_empty() {
            true => Visibility::Hidden,
            false => Visibility::Inherited,
        });
        if text_node.0 != text {
            text_node.0 = text;
        }
    }
    #[cfg(target_family = "wasm")]
    if *shown != text {
        let element = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("teams"));
        if let Some(element) = element {
            element.set_text_content(Some(&text));
        }
        *shown = text;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screen_toggles_switches_and_balances() {
        let mut s = Screen::default();
        // Closed: the stick moves the player, not the team.
        assert_eq!(s.step((0, 80, false), 0, 2, 0), None);
        assert_eq!(s.step((0, 0, true), 0, 2, 0), None);
        assert!(s.open);
        // A push right: the next team (wrapping), held: no repeat; left from 0 of 4: team 3.
        assert_eq!(s.step((0, 80, true), 1, 2, 0), Some(Teams::Switch(0)));
        assert_eq!(s.step((0, 80, false), 1, 2, 0), None);
        assert_eq!(s.step((0, 0, false), 0, 4, 0), None);
        assert_eq!(s.step((0, -80, false), 0, 4, 0), Some(Teams::Switch(3)));
        // C-left adds to the BALANCE, C-right takes; only with 2 teams.
        assert_eq!(s.step((C_LEFT, 0, false), 0, 2, 2), Some(Teams::Balance(3)));
        assert_eq!(
            s.step((C_RIGHT, 0, false), 0, 2, 2),
            Some(Teams::Balance(1))
        );
        assert_eq!(s.step((0, 0, false), 0, 4, 0), None);
        assert_eq!(s.step((C_LEFT, 0, false), 0, 4, 0), None);
        // A closes it; Tab again reopens.
        assert_eq!(s.step((BUTTON_A, 0, false), 0, 2, 0), None);
        assert!(!s.open);
        assert_eq!(s.step((0, 0, true), 0, 2, 0), None);
        assert!(s.open);
    }

    /// The board's lines, each run of spaces one (the columns' widths aside).
    fn lines(text: &str) -> Vec<String> {
        (text.lines())
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect()
    }

    fn player(slot: u16, team: u8, kills: u16, deaths: u16) -> Combat {
        Combat {
            slot,
            team,
            kills,
            deaths,
            ..Combat::default()
        }
    }

    #[test]
    fn the_board_lists_each_team_by_name() {
        let m = MatchView {
            teams: 2,
            balance: -1,
            raptor: true,
            scores: [2, 3],
            ..MatchView::default()
        };
        let (ana, own) = (player(0, 0, 1, 2), player(1, 1, 4, 0));
        let (bot2, bot3) = (player(2, 0, 3, 1), player(3, 1, 0, 5));
        let all = [
            (&ana, Some("Ana")),
            (&own, Some("Sylas")),
            (&bot2, None),
            (&bot3, None),
        ];
        assert_eq!(
            lines(&text(&m, &own, &all)),
            [
                "WAR: RAPTOR",
                "NAME KILLS DEATHS",
                "UGAS 2 POINTS",
                "BOT 3 3 1",
                "Ana 1 2",
                ">RAPTORS 3 POINTS",
                "> Sylas 4 0",
                "BOT 4 0 5",
                "BALANCE -1 (W/S)",
                "stick: change team, A: close",
            ]
        );
    }

    #[test]
    fn free_for_all_ranks_everyone_with_their_lives() {
        let m = MatchView {
            kill_limit: 10,
            ..MatchView::default()
        };
        let own = Combat {
            lives_left: 2,
            ..player(0, 0, 2, 1)
        };
        let bot = Combat {
            lives_left: 3,
            ..player(1, 1, 5, 0)
        };
        let ana = Combat {
            out: 3,
            ..player(2, 2, 0, 3)
        };
        let all = [(&own, Some("Sylas")), (&bot, None), (&ana, Some("Ana"))];
        let text = text(&m, &own, &all);
        assert_eq!(
            lines(&text),
            [
                "DEATHMATCH FIRST TO 10",
                "NAME KILLS DEATHS LIVES",
                "1ST BOT 2 5 0 3",
                ">2ND Sylas 2 1 2",
                "3RD Ana 0 3 OUT",
                "Tab: close",
            ]
        );
        // The columns line up: every row as long as the header.
        let rows: Vec<_> = text.lines().skip(1).take(4).map(str::len).collect();
        assert!(rows.iter().all(|&n| n == rows[0]), "{rows:?}");
        // A long pseudonym is cut to the column.
        let long = "A".repeat(30);
        let cut = super::text(&m, &own, &[(&own, Some(&long))]);
        assert!(cut.contains(&format!(">1ST {} ", "A".repeat(NAME))));
        assert!(!cut.contains(&"A".repeat(NAME + 1)));
    }

    #[test]
    fn race_shows_the_race_table() {
        let mut m = MatchView {
            race: true,
            ..MatchView::default()
        };
        m.racing.laps = 3;
        m.racing.racers = vec![
            RacerView {
                slot: 0,
                place: 2,
                lap: 1,
                best: 2550,
                ..RacerView::default()
            },
            RacerView {
                slot: 1,
                place: 1,
                lap: 2,
                order: 1,
                best: -1,
                ..RacerView::default()
            },
        ];
        let (own, bot) = (player(0, 0, 0, 0), player(1, 1, 0, 0));
        assert_eq!(
            lines(&text(&m, &own, &[(&own, Some("Sylas")), (&bot, None)])),
            [
                "RACE",
                "NAME LAP BEST",
                "1ST BOT 2 DONE -",
                ">2ND Sylas 2/3 0:42.50",
                "Tab: close",
            ]
        );
    }

    #[test]
    fn the_board_opens_in_every_mode_but_stands_the_player_only_on_teams() {
        let mut s = Screen::default();
        assert_eq!(s.step((0, 0, true), 0, 0, 0), None);
        assert!(s.open && !s.picking());
        // Free for all, the stick and A play on.
        assert_eq!(s.step((BUTTON_A, 80, true), 0, 0, 0), None);
        assert!(s.open);
        s.teams = 2;
        assert!(s.picking());
    }
}
