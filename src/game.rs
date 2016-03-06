
use rand::{self, Rng};
use std::convert::From;
use std::collections::HashSet;
use std::collections::HashMap;
use std::fmt;

use info::*;

/*
* Type definitions
*/

pub type Color = &'static str;
pub const COLORS: [Color; 5] = ["blue", "red", "yellow", "white", "green"];

pub type Value = u32;
// list of (value, count) pairs
pub const VALUES : [Value; 5] = [1, 2, 3, 4, 5];
pub const VALUE_COUNTS : [(Value, u32); 5] = [(1, 3), (2, 2), (3, 2), (4, 2), (5, 1)];
pub const FINAL_VALUE : Value = 5;

pub struct Card {
    pub color: Color,
    pub value: Value,
}
impl fmt::Debug for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.color, self.value)
    }
}

#[derive(Debug)]
// basically a stack of cards, or card info
pub struct Pile<T>(Vec<T>);
impl <T> Pile<T> {
    pub fn new() -> Pile<T> {
        Pile(Vec::<T>::new())
    }
    pub fn draw(&mut self) -> Option<T> {
        self.0.pop()
    }
    pub fn place(&mut self, item: T) {
        self.0.push(item);
    }
    pub fn take(&mut self, index: usize) -> T {
        self.0.remove(index)
    }
    pub fn top(&self) -> Option<&T> {
        self.0.last()
    }
    pub fn shuffle(&mut self) {
        rand::thread_rng().shuffle(&mut self.0[..]);
    }
    pub fn size(&self) -> usize {
        self.0.len()
    }
}
impl <T> From<Vec<T>> for Pile<T> {
    fn from(items: Vec<T>) -> Pile<T> {
        Pile(items)
    }
}

pub type Cards = Pile<Card>;

pub type CardsInfo = Pile<CardInfo>;

pub type Player = u32;

#[derive(Debug)]
pub enum Hint {
    Color,
    Value,
}

// represents the choice a player made in a given turn
#[derive(Debug)]
pub enum TurnChoice {
    Hint,
    Discard(usize),
    Play(usize),
}

// represents a turn taken in the game
pub struct Turn<'a> {
    pub player: &'a Player,
    pub choice: &'a TurnChoice,
}

// represents possible settings for the game
pub struct GameOptions {
    pub num_players: u32,
    pub hand_size: u32,
    // when hits 0, you cannot hint
    pub num_hints: u32,
    // when hits 0, you lose
    pub num_lives: u32,
}

// The state of a given player:  all other players may see this
#[derive(Debug)]
pub struct PlayerState {
    // the player's actual hand
    pub hand: Cards,
    // represents what is common knowledge about the player's hand
    pub info: CardsInfo,
}

// State of everything except the player's hands
// Is all completely common knowledge
#[derive(Debug)]
pub struct BoardState {
    deck: Cards,
    pub discard: Cards,
    pub fireworks: HashMap<Color, Cards>,

    pub num_players: u32,

    // which turn is it?
    pub turn: u32,
    // // whose turn is it?
    pub player: Player,

    pub hints_total: u32,
    pub hints_remaining: u32,
    pub lives_total: u32,
    pub lives_remaining: u32,
    // only relevant when deck runs out
    deckless_turns_remaining: u32,
}

// complete game view of a given player
// state will be borrowed GameState
#[derive(Debug)]
pub struct GameStateView<'a> {
    // the player whose view it is
    pub player: Player,
    // what is known about their own hand (and thus common knowledge)
    pub info: &'a CardsInfo,
    // the cards of the other players, as well as the information they have
    pub other_player_states: HashMap<Player, &'a PlayerState>,
    // board state
    pub board: &'a BoardState,
}

// complete game state (known to nobody!)
#[derive(Debug)]
pub struct GameState {
    pub player_states: HashMap<Player, PlayerState>,
    pub board: BoardState,
}

pub type Score = u32;

impl GameState {
    pub fn new(opts: &GameOptions) -> GameState {
        let mut deck = GameState::make_deck();

        let mut player_states : HashMap<Player, PlayerState> = HashMap::new();
        for i in 0..opts.num_players {
            let raw_hand = (0..opts.hand_size).map(|_| {
                    // we can assume the deck is big enough to draw initial hands
                    deck.draw().unwrap()
                }).collect::<Vec<_>>();
            let infos = (0..opts.hand_size).map(|_| {
                CardInfo::new()
            }).collect::<Vec<_>>();
            let state = PlayerState {
                hand: Cards::from(raw_hand),
                info: CardsInfo::from(infos),
            };
            player_states.insert(i,  state);
        }

        let mut fireworks : HashMap<Color, Cards> = HashMap::new();
        for color in COLORS.iter() {
            let mut firework = Cards::new();
            let card = Card { value: 0, color: color };
            firework.place(card);
            fireworks.insert(color, firework);
        }

        GameState {
            player_states: player_states,
            board: BoardState {
                deck: deck,
                fireworks: fireworks,
                discard: Cards::new(),
                num_players: opts.num_players,
                player: 0,
                turn: 1,
                hints_total: opts.num_hints,
                hints_remaining: opts.num_hints,
                lives_total: opts.num_lives,
                lives_remaining: opts.num_lives,
                // number of turns to play with deck length ran out
                deckless_turns_remaining: opts.num_players + 1,
            }
        }
    }

    fn make_deck() -> Cards {
        let mut deck: Cards = Cards::from(Vec::new());

        for color in COLORS.iter() {
            for &(value, count) in VALUE_COUNTS.iter() {
                for _ in 0..count {
                    deck.place(Card {color: color, value: value});
                }
            }
        };
        deck.shuffle();
        info!("Created deck: {:?}", deck);
        deck
    }

    pub fn get_players(&self) -> Vec<Player> {
        (0..self.board.num_players).collect::<Vec<_>>()
    }

    pub fn is_over(&self) -> bool {
        // TODO: add condition that fireworks cannot be further completed?
        (self.board.lives_remaining == 0) ||
        (self.board.deckless_turns_remaining == 0)
    }

    pub fn score(&self) -> Score {
        let mut score = 0;
        for (_, firework) in &self.board.fireworks {
            // subtract one to account for the 0 we pushed
            score += firework.size() - 1;
        }
        score as u32
    }

    // get the game state view of a particular player
    pub fn get_view(&self, player: Player) -> GameStateView {
        let mut other_player_states = HashMap::new();
        for (other_player, state) in &self.player_states {
            if player != *other_player {
                other_player_states.insert(player, state);
            }
        }
        GameStateView {
            player: player,
            info: &self.player_states.get(&player).unwrap().info,
            other_player_states: other_player_states,
            board: &self.board,
        }
    }

    // takes a card from the player's hand, and replaces it if possible
    fn take_from_hand(&mut self, index: usize) -> Card {
        let ref mut state = self.player_states.get_mut(&self.board.player).unwrap();
        let card = state.hand.take(index);
        state.info.take(index);
        if let Some(new_card) = self.board.deck.draw() {
            state.hand.place(new_card);
            state.info.place(CardInfo::new());
        }
        card
    }

    fn try_add_hint(&mut self) {
        if self.board.hints_remaining < self.board.hints_total {
            self.board.hints_remaining += 1;
        }
    }

    pub fn process_choice(&mut self, choice: &TurnChoice) {
        match *choice {
            TurnChoice::Hint => {
                assert!(self.board.hints_remaining > 0);
                self.board.hints_remaining -= 1;
                // TODO: actually inform player of values..
                // nothing to update, really...
                // TODO: manage common knowledge
            }
            TurnChoice::Discard(index) => {
                let card = self.take_from_hand(index);
                self.board.discard.place(card);

                self.try_add_hint();
            }
            TurnChoice::Play(index) => {
                let card = self.take_from_hand(index);

                debug!(
                    "Here!  Playing card at {}, which is {:?}",
                    index, card
                );

                let mut firework_made = false;

                {
                    let ref mut firework = self.board.fireworks.get_mut(&card.color).unwrap();

                    let playable = {
                        let under_card = firework.top().unwrap();
                        card.value == under_card.value + 1
                    };

                    if playable {
                        firework_made = card.value == FINAL_VALUE;
                        firework.place(card);
                    } else {
                        self.board.discard.place(card);
                        self.board.lives_remaining -= 1;
                        debug!(
                            "Removing a life! Lives remaining: {}",
                            self.board.lives_remaining
                        );
                    }
                }

                if firework_made {
                    self.try_add_hint();
                }
            }
        }

        if self.board.deck.size() == 0 {
            self.board.deckless_turns_remaining -= 1;
        }
        self.board.turn += 1;
        self.board.player = (self.board.player + 1) % self.board.num_players;
        assert_eq!((self.board.turn - 1) % self.board.num_players, self.board.player);

    }
}
