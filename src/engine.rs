use chess::MoveGen;
use chess::ChessMove;
use chess::BoardStatus;
use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;
use rayon::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;

pub struct Engine {
    brett: chess::Board,
}

impl Engine {
    pub fn new(brett: chess::Board) -> Self {
        Engine {
            brett,
        }
    }

    pub fn minmax(brett: &chess::Board, fifty_move_counter: u8, depth:u64, max_depth: u64, mut alpha: f32, mut beta: f32, maximizing_player: bool, eval_map: &mut HashMap<u64, f32>, depth_counter: u64) -> f32 {
        if fifty_move_counter >= 50 {
            return 0.0;
        }
        match brett.status() { 
            BoardStatus::Ongoing => () , 
            BoardStatus::Stalemate => {
                let eval = 0.0;   
                eval_map.insert(brett.get_hash(), eval);
                return eval;
            }
            BoardStatus::Checkmate =>  {
                let bonus = match maximizing_player {
                    true  => -1.0 * (4000.0 / depth_counter as f32),
                    false =>  1.0 * (4000.0 / depth_counter as f32),
                };
                let eval = Engine::eval_board(brett, fifty_move_counter) + bonus;   
                return eval;
            }
        }
        if eval_map.contains_key(&brett.get_hash()) {
            return eval_map[&brett.get_hash()];
        }
        if depth == 0 || depth_counter >= max_depth {
            let eval = Engine::eval_board(brett, fifty_move_counter);   
            return eval;
        }

        let mut moves = MoveGen::new_legal(brett).collect::<Vec<chess::ChessMove>>();
        let mut rng = thread_rng();
        moves.shuffle(&mut rng);
        if maximizing_player {
            let mut max_eval = f32::NEG_INFINITY;
            for m in moves {
                let fifty = if brett.piece_on(m.get_source()) == Some(chess::Piece::Pawn) || brett.piece_on(m.get_dest()) != None {
                    0
                } else {
                    fifty_move_counter + 1
                };
                let eval = if depth == 1 {
                    if brett.piece_on(m.get_dest()) != None {
                        let brett = brett.make_move_new(m);
                        Engine::minmax( &brett, fifty, 1, max_depth ,alpha, beta, false,eval_map, depth_counter + 1)
                    } else {
                        let brett = brett.make_move_new(m);
                        if brett.checkers().0 != 0 {
                            Engine::minmax( &brett, fifty, 2, max_depth ,alpha, beta, false,eval_map, depth_counter + 1)
                        } else {
                            Engine::minmax( &brett, fifty, depth - 1, max_depth ,alpha, beta, false,eval_map, depth_counter + 1)
                        }
                    }
                } else {
                    let brett = brett.make_move_new(m);
                    Engine::minmax( &brett, fifty, depth - 1, max_depth ,alpha, beta, false,eval_map, depth_counter + 1)
                };                
                if eval < 1000.0 && eval > -1000.0 {
                    eval_map.insert(brett.get_hash(), eval);
                }   
                max_eval = max_eval.max(eval);
                alpha = alpha.max(eval);
                if beta <= alpha {
                    break;
                }
            }   
            return max_eval;
        } else {
            let mut min_eval = f32::INFINITY;
            for m in moves {
                let fifty = if brett.piece_on(m.get_source()) == Some(chess::Piece::Pawn) || brett.piece_on(m.get_dest()) != None {
                    0
                } else {
                    fifty_move_counter + 1
                };
                let eval = if depth == 1 {
                    if brett.piece_on(m.get_dest()) != None {
                        let brett = brett.make_move_new(m);
                        Engine::minmax( &brett, fifty, 1, max_depth ,alpha, beta, true,eval_map, depth_counter + 1)
                    } else {
                        let brett = brett.make_move_new(m);
                        if brett.checkers().0 != 0 {
                            Engine::minmax( &brett, fifty, 2, max_depth ,alpha, beta, true,eval_map, depth_counter + 1)
                        } else {
                            Engine::minmax( &brett, fifty, depth - 1, max_depth ,alpha, beta, true,eval_map, depth_counter + 1)
                        }
                    }
                } else {
                    let brett = brett.make_move_new(m);
                    Engine::minmax( &brett, fifty, depth - 1, max_depth ,alpha, beta, true,eval_map, depth_counter + 1)
                };  
                if eval < 1000.0 && eval > -1000.0 {
                    eval_map.insert(brett.get_hash(), eval);
                }              
                min_eval = min_eval.min(eval);
                beta = beta.min(eval);
                if beta <= alpha {
                    break;
                }
            }
            return min_eval;
        } 
    }   

    pub fn best_move(&self, mut fifty_move_counter: u8, depth: u64, max_depth: u64, time: u32, start: SystemTime) -> (ChessMove, u8) {
        let mut best = f32::MIN;
        
        let maximizing_player = match self.brett.side_to_move() {
            chess::Color::Black => true,
            chess::Color::White => false,
        };
        let factor = match self.brett.side_to_move() {
            chess::Color::Black => -1.0,
            chess::Color::White =>  1.0,
        };
        
        let all_moves = chess::MoveGen::new_legal(&self.brett).collect::<Vec<chess::ChessMove>>();
        let mut best_move = all_moves[0];
        let mut moves: Vec<(f32, chess::ChessMove)> = Vec::new();
        all_moves.par_iter()
        .map(|m| {
            let fifty = if self.brett.piece_on(m.get_source()) == Some(chess::Piece::Pawn) || self.brett.piece_on(m.get_dest()) != None  {
                0
            } else {
                fifty_move_counter + 1
            };
            let brett = self.brett.make_move_new(*m);
            let mut eval_map:HashMap<u64, f32>  = HashMap::new();
            let eval = match self.brett.status() { 
                BoardStatus::Stalemate => 0.0,
                BoardStatus::Checkmate => 100000.0,
                BoardStatus::Ongoing => {
                    factor * Engine::minmax(&brett, fifty, depth - 1, max_depth, f32::NEG_INFINITY, f32::INFINITY, maximizing_player, &mut eval_map, 2)
                },
            };
            (eval,*m)
        }).collect_into_vec(&mut moves);
        let mut rng = thread_rng();
        moves.shuffle(&mut rng);
        
        for (eval,m) in moves {
            if eval > best {
                best = eval;
                best_move = m;
            }
        }

        if SystemTime::now() < start + Duration::new(0,time) && depth < 50 {
            if max_depth <= depth + 6 {
                return self.best_move(fifty_move_counter, depth, max_depth+2, time,start);
            }
            return self.best_move(fifty_move_counter, depth + 2, max_depth, time,start);
        }
        
        if self.brett.piece_on(best_move.get_source()) == Some(chess::Piece::Pawn) || self.brett.piece_on(best_move.get_dest()) != None {
            fifty_move_counter = 0;
        } else {
            fifty_move_counter += 1;
        }

        Engine::print_move(best_move, &self.brett);
        println!("tiefe: {}, max tiefe: {}, eval: {:.2}, time: {:?}", depth, max_depth, factor * best, SystemTime::now().duration_since(start).unwrap());

        (best_move, fifty_move_counter)
    }

    fn eval_board(brett: &chess::Board, fifty_move_counter: u8) -> f32 {
        let outcome = brett.status();
        match outcome {
            chess::BoardStatus::Checkmate => {
                if brett.side_to_move() == chess::Color::White {
                    return -1000.0;
                } else {
                    return  1000.0;
                }
            },
            chess::BoardStatus::Stalemate => {
                return 0.0;
            },
            chess::BoardStatus::Ongoing => {},
        }
        if fifty_move_counter >= 50 {
            return 0.0;
        }
        let mut eval = 0.0;
        for square in chess::ALL_SQUARES {
            if let (Some(p), Some(c)) = (brett.piece_on(square), brett.color_on(square)) {
                eval += match (p,c) {
                    (chess::Piece::Pawn, chess::Color::White) => 1.0,
                    (chess::Piece::Pawn, chess::Color::Black) => -1.0,
                    (chess::Piece::Knight, chess::Color::White) => 3.05,
                    (chess::Piece::Knight, chess::Color::Black) => -3.05,
                    (chess::Piece::Bishop, chess::Color::White) => 3.33,
                    (chess::Piece::Bishop, chess::Color::Black) => -3.33,
                    (chess::Piece::Rook, chess::Color::White) => 5.63,
                    (chess::Piece::Rook, chess::Color::Black) => -5.63,
                    (chess::Piece::Queen, chess::Color::White) => 9.5,
                    (chess::Piece::Queen, chess::Color::Black) => -9.5,
                    (chess::Piece::King, chess::Color::White) => 0.0,
                    (chess::Piece::King, chess::Color::Black) => -0.0,
                }
            }
        }   
        eval
    }

    fn print_move(input: ChessMove, brett: &chess::Board) {
        if input.get_source() == chess::Square::E1 && input.get_dest() == chess::Square::G1 {
            println!("{:?} spielt: {}",brett.side_to_move() , "O-O");
            return;
        }
        if input.get_source() == chess::Square::E1 && input.get_dest() == chess::Square::C1 {
            println!("{:?} spielt: {}",brett.side_to_move() , "O-O-O");
            return;
        }
        if input.get_source() == chess::Square::E8 && input.get_dest() == chess::Square::G8 {
            println!("{:?} spielt: {}",brett.side_to_move() , "O-O");
            return;
        }
        if input.get_source() == chess::Square::E8 && input.get_dest() == chess::Square::C8 {
            println!("{:?} spielt: {}",brett.side_to_move() , "O-O-O");
            return;
        }
        let piece = match brett.piece_on(input.get_source()).unwrap() {
            chess::Piece::Pawn => san_rs::Piece::Pawn,
            chess::Piece::Knight => san_rs::Piece::Knight,
            chess::Piece::Bishop => san_rs::Piece::Bishop,
            chess::Piece::Rook => san_rs::Piece::Rook,
            chess::Piece::Queen => san_rs::Piece::Queen,
            chess::Piece::King => san_rs::Piece::King,
        };
    
        let source = san_rs::Position::new(Some(input.get_source().get_file().to_index() as usize), Some(7 - input.get_source().get_rank().to_index() as usize));
    
        let target: san_rs::Position = san_rs::Position::new(Some(input.get_dest().get_file().to_index() as usize), Some(7 - input.get_dest().get_rank().to_index() as usize)); 
        let move_kind = san_rs::MoveKind::Normal(source, target); 
        let mut m = san_rs::Move::new(piece, move_kind);
        if (input.get_dest().get_rank().to_index() == 0 || input.get_dest().get_rank().to_index() == 7) && brett.piece_on(input.get_source()).unwrap() == chess::Piece::Pawn {
            let promotion = match input.get_promotion() {
                Some(chess::Piece::Queen) => Some(san_rs::Piece::Queen),
                Some(chess::Piece::Rook) => Some(san_rs::Piece::Rook),
                Some(chess::Piece::Bishop) => Some(san_rs::Piece::Bishop),
                Some(chess::Piece::Knight) => Some(san_rs::Piece::Knight),
                _ => None,
            };
            m.promotion = promotion; 
        }
        m.is_capture = brett.piece_on(input.get_dest()) != None;
    
        if brett.checkers().0 != 0 {
            m.check_type = Some(san_rs::CheckType::Check);
        }
            
        m.check_type = match brett.status() {
            BoardStatus::Checkmate => Some(san_rs::CheckType::Mate),
            _ => None,
        };
        let san_string = m.compile();
        println!("{:?} spielt: {}",brett.side_to_move() , san_string);
    }

}