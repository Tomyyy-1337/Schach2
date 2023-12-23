extern crate sdl2;

use chess::Board;
use chess::ChessMove;
use chess::Square;
use chess::BoardStatus;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::rect::Point;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;
pub mod texture_manager;
pub mod engine;
// use std::str::FromStr;


const SQUARE_SIZE:u32 = 100;

fn main() -> Result<(), String> {
    rayon::ThreadPoolBuilder::new().num_threads(18).build_global().unwrap();
    
    let mut brett = chess::Board::default();
    // let mut brett = chess::Board::from_str("4k3/1r6/3q4/8/8/8/5Q2/4K3 w - - 0 1").unwrap();
    let mut fifty_move_counter: u8 = 0;

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("Schach", SQUARE_SIZE * 8, SQUARE_SIZE * 8)
    .position_centered()
    .build()
    .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator: sdl2::render::TextureCreator<sdl2::video::WindowContext> = canvas.texture_creator();
    let mut tex_man: texture_manager::ResourceManager<'_, String, sdl2::render::Texture<'_>, sdl2::render::TextureCreator<sdl2::video::WindowContext>> = texture_manager::TextureManager::new(&texture_creator);

    tex_man.load("img/black-bishop.png")?;
    tex_man.load("img/black-king.png")?;
    tex_man.load("img/black-queen.png")?;
    tex_man.load("img/black-knight.png")?;
    tex_man.load("img/black-pawn.png")?;
    tex_man.load("img/black-rook.png")?;
    tex_man.load("img/white-bishop.png")?;
    tex_man.load("img/white-king.png")?;
    tex_man.load("img/white-queen.png")?;
    tex_man.load("img/white-knight.png")?;
    tex_man.load("img/white-pawn.png")?;
    tex_man.load("img/white-rook.png")?;

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut legal_moves: Vec<(i32,i32)> = Vec::new();
    let mut active_piece: Option<(i32, i32)> = None;

    let mut calulation_running = false;
    let mut calculation_end = SystemTime::now();
    let mut rx: mpsc::Receiver<(ChessMove, u8)> = mpsc::channel().1;
    let mut tx: mpsc::Sender<(ChessMove, u8)>;
    let mut waiting = false;

    'running: loop {
        canvas.clear();

        if fifty_move_counter >= 51 {
            if !waiting {
                waiting = true;
                println!("Unentschieden");
            }
            if calculation_end.elapsed().unwrap().as_secs() > 10 {
                legal_moves.clear();
                brett = Board::default();
                waiting = false;
                fifty_move_counter = 0;
            }
        } else {
            match brett.status(){
                BoardStatus::Ongoing => {
                    if let Ok(m) = rx.try_recv() {
                        calculation_end = SystemTime::now();
                        calulation_running = false;
                        legal_moves.clear();
                        brett = brett.make_move_new(m.0);
                        legal_moves.push((m.0.get_source().get_file().to_index() as i32, m.0.get_source().get_rank().to_index() as i32));
                        legal_moves.push((m.0.get_dest().get_file().to_index() as i32, m.0.get_dest().get_rank().to_index() as i32));
                        fifty_move_counter = m.1;
                    } else if !calulation_running && brett.side_to_move() == chess::Color::Black {
                        calulation_running = true;
                        (tx, rx) = std::sync::mpsc::channel();
                        thread::spawn(move || {
                            let engine = engine::Engine::new(brett);
                            let m = engine.best_move(fifty_move_counter, 4, 10, 1_000_000_000 / 8, SystemTime::now()); 
                            tx.send(m).unwrap();
                        });
                    }
                },
                BoardStatus::Checkmate => {
                    if !waiting {
                        waiting = true;
                        match brett.side_to_move() {
                            chess::Color::White => println!("Schwarz gewinnt"),
                            chess::Color::Black => println!("Weiß gewinnt"),
                        }
                    }
                    if calculation_end.elapsed().unwrap().as_secs() > 10 {
                        legal_moves.clear();
                        brett = Board::default();
                        waiting = false;
                    }
                },
                BoardStatus::Stalemate => {
                    if !waiting {
                        waiting = true;
                        println!("Unentschieden");
                    }
                    if calculation_end.elapsed().unwrap().as_secs() > 10 {
                        legal_moves.clear();
                        brett = Board::default();
                        waiting = false;
                    }
                },
            }
        }


        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                },
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    calulation_running = false;
                    waiting = false;
                    brett = chess::Board::default();
                    legal_moves.clear();
                },
                Event::MouseButtonDown { mouse_btn, x, y, .. } => {
                    let x = x / SQUARE_SIZE as i32 ;
                    let y = 7 - y / SQUARE_SIZE as i32;
                    match mouse_btn {
                        MouseButton::Left => {
                            if active_piece.is_none() {
                                legal_moves.clear();
                                active_piece = Some((x, y));
                                let moves = chess::MoveGen::new_legal(&brett);
                                for m in moves {
                                    let start = (m.get_source().get_file().to_index() as i32, m.get_source().get_rank().to_index() as i32);
                                    let end = (m.get_dest().get_file().to_index() as i32, m.get_dest().get_rank().to_index() as i32);
                                    if start == (x, y) {
                                        legal_moves.push(end);
                                    }
                                }
                            } else {
                                let start = active_piece.unwrap();
                                let end = (x, y);
                                if legal_moves.contains(&end) {
                                    let chess_move = if (end.1 == 0 || end.1 == 7) && brett.piece_on(Square::make_square(chess::Rank::from_index(start.1 as usize), chess::File::from_index(start.0 as usize))).unwrap() == chess::Piece::Pawn {
                                        ChessMove::new(Square::make_square(chess::Rank::from_index(start.1 as usize), chess::File::from_index(start.0 as usize)), Square::make_square(chess::Rank::from_index(end.1 as usize), chess::File::from_index(end.0 as usize)), Some(chess::Piece::Queen))
                                    } else {
                                        ChessMove::new(Square::make_square(chess::Rank::from_index(start.1 as usize), chess::File::from_index(start.0 as usize)), Square::make_square(chess::Rank::from_index(end.1 as usize), chess::File::from_index(end.0 as usize)), None)
                                    };
                                    legal_moves.clear();
                                    legal_moves.push(start);
                                    legal_moves.push(end);
                                    brett = brett.make_move_new(chess_move);
                                    active_piece = None;
                                } else {
                                    legal_moves.clear();
                                    active_piece = Some((x, y));
                                    let moves = chess::MoveGen::new_legal(&brett);
                                    for m in moves {
                                        let start = (m.get_source().get_file().to_index() as i32, m.get_source().get_rank().to_index() as i32);
                                        let end = (m.get_dest().get_file().to_index() as i32, m.get_dest().get_rank().to_index() as i32);
                                        if start == (x, y) {
                                            legal_moves.push(end);
                                        }
                                    }
                                } 
                            }
                        }
                        _ => {}
                    }
                },
                _ => {}
            }
        }

        //Brett
        for i in 0..8 {
            for j in 0..8 {
                let x = i * SQUARE_SIZE;
                let y = j * SQUARE_SIZE;
                
                let color = if (i + j) % 2 == 0 && legal_moves.contains(&(i as i32, 7 - j as i32)) {
                    //Color::RGB(255, 150, 150)
                    Color::RGB(36, 158, 108)
                } else if (i + j) % 2 == 1 && legal_moves.contains(&(i as i32, 7 - j as i32)) {
                    // Color::RGB(100, 70, 30) 
                    Color::RGB(38, 89, 68)
                } else  if (i + j) % 2 == 0 {
                    // Color::RGB(255, 255, 255) 
                    Color::RGB(231,206,181)
                } else {
                    // Color::RGB(20, 100, 20) 
                    Color::RGB(101,48,36)
                };
                canvas.set_draw_color(color);
                canvas.fill_rect(sdl2::rect::Rect::new(x as i32, y as i32, SQUARE_SIZE, SQUARE_SIZE)).unwrap();
            }
        }

        for square in chess::ALL_SQUARES {
            if let (Some(p), Some(c)) = (brett.piece_on(square), brett.color_on(square)) {
                let texture_name = match (p,c) {
                    (chess::Piece::Pawn, chess::Color::White) => "img/white-pawn.png",
                    (chess::Piece::Pawn, chess::Color::Black) => "img/black-pawn.png",
                    (chess::Piece::Knight, chess::Color::White) => "img/white-knight.png",
                    (chess::Piece::Knight, chess::Color::Black) => "img/black-knight.png",
                    (chess::Piece::Bishop, chess::Color::White) => "img/white-bishop.png",
                    (chess::Piece::Bishop, chess::Color::Black) => "img/black-bishop.png",
                    (chess::Piece::Rook, chess::Color::White) => "img/white-rook.png",
                    (chess::Piece::Rook, chess::Color::Black) => "img/black-rook.png",
                    (chess::Piece::Queen, chess::Color::White) => "img/white-queen.png",
                    (chess::Piece::Queen, chess::Color::Black) => "img/black-queen.png",
                    (chess::Piece::King, chess::Color::White) => "img/white-king.png",
                    (chess::Piece::King, chess::Color::Black) => "img/black-king.png",
                };
                let img_size = 128;
                let texture = tex_man.load(&texture_name)?;
                let src = Rect::new(0,0,img_size,img_size);
                let x: i32 = (square.get_file().to_index() as u32 * SQUARE_SIZE) as i32;
                let y: i32 = ((7 - square.get_rank().to_index()) as u32 * SQUARE_SIZE) as i32;
                let dest = Rect::new(x,y,SQUARE_SIZE,SQUARE_SIZE);
                let center = Point::new( 0,0);

                canvas.copy_ex(
                    &texture, 
                    src,  
                    dest,
                    0.0,
                    center, 
                    false, 
                    false 
                )?;          
                
            }
        }

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 120));
    }
    
    Ok(())
}
