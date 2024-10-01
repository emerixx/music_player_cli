use crossterm::cursor::{self, MoveToPreviousLine, MoveUp};
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::style::Print;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use rodio::decoder::{Decoder, DecoderError};
use rodio::source::{self, SineWave, Source};
use rodio::{OutputStream, Sample, Sink};
use std::fs::File;
use std::io::BufReader;
use std::io::{stdin, stdout, Write};
use std::time::Duration;
use std::{fs, io, io::prelude::*, process};
use std::{thread, time};

#[macro_use]
extern crate crossterm;

use crossterm::{
    event::{
        poll, read, DisableBracketedPaste, DisableFocusChange, DisableMouseCapture,
        EnableBracketedPaste, EnableFocusChange, EnableMouseCapture, Event,
    },
    execute,
};

static DIR: &str = "./";
static CHANGE_VOL_BY: f32 = 0.1;
static VOL_MAX: f32 = 2.5;
static VOL_MIN: f32 = 0.0;
static N_OF_LINES: usize = 4;

fn main() {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    let mut play_random = true;
    let mut random_queue: Vec<usize> = generate_random_queue();
    let mut index_in_rand_queue: usize = 0;
    let mut current_song: usize = 0;
    let mut current_song_len_str: String = "".to_owned();
    let mut string_to_print: String = "".to_owned();
    let song_names: Vec<String> = list_music_files();
    for song_name in song_names.iter() {
        println!("{}", song_name);
    }
    println!("----------");
    for n in 0..N_OF_LINES {
        print!("\n");
    }
    sink.set_volume(0.5);

    enable_raw_mode().unwrap();
    execute!(
        std::io::stdout(),
        EnableBracketedPaste,
        EnableFocusChange,
        EnableMouseCapture
    );
    loop {
        // `poll()` waits for an `Event` for a given time period
        if poll(Duration::from_millis(100)).unwrap() {
            // It's guaranteed that the `read()` won't block when the `poll()`
            // function returns `true`
            match read().unwrap() {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                }) => break,
                Event::Key(KeyEvent {
                    code: KeyCode::Char(' '),
                    ..
                }) => toggle_pause(&sink),
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    ..
                }) => next_song(&sink),
                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    ..
                }) => {
                    if index_in_rand_queue >= 1 {
                        index_in_rand_queue -= 1;
                    } else {
                        index_in_rand_queue = random_queue.len() - 1
                    }
                    current_song = random_queue[index_in_rand_queue];
                    current_song_len_str =
                        get_file_duration(&(song_names[random_queue[index_in_rand_queue]]));

                    add_file_to_empty_sink(&sink, &(song_names[random_queue[index_in_rand_queue]]));
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Up, ..
                }) => increase_volume(&sink),
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    ..
                }) => decrease_volume(&sink),
                Event::Key(KeyEvent {
                    code: KeyCode::Char('r'),
                    ..
                }) => random_queue = generate_random_queue(),
                _ => (),
            }
        } else {
            if sink.empty() {
                if play_random {
                    if index_in_rand_queue == random_queue.len() + 1 {
                        index_in_rand_queue = 0;
                    } else {
                        index_in_rand_queue += 1;
                    }
                    current_song = random_queue[index_in_rand_queue];
                    current_song_len_str = get_file_duration(&(song_names[current_song]));
                    add_file_to_empty_sink(&sink, &(song_names[current_song]))
                }
            }
            string_to_print = "".to_owned();
            //first line
            string_to_print = string_to_print
                + "Current song: "
                + &(song_names[current_song])
                + " - "
                + get_current_song_played_duration(&sink).as_str()
                + "/"
                + current_song_len_str.as_str()
                + "\r\n";
            //second line
            string_to_print = string_to_print
                + "Next song: "
                + &song_names[random_queue[index_in_rand_queue + 1]]
                + " - "
                + get_file_duration(&(song_names[random_queue[index_in_rand_queue + 1]])).as_str()
                + "\r\n";
            //third line

            if index_in_rand_queue >= 1 {
                string_to_print = string_to_print
                    + "Prev song: "
                    + song_names[random_queue[index_in_rand_queue - 1]].as_str()
                    + " - "
                    + get_file_duration(&(song_names[random_queue[index_in_rand_queue - 1]]))
                        .as_str()
                    + "\r\n";
            } else {
                string_to_print = string_to_print
                    + "Prev song: "
                    + song_names[random_queue[random_queue.len() - 1]].as_str()
                    + " - "
                    + get_file_duration(&(song_names[random_queue[random_queue.len() - 1]]))
                        .as_str()
                    + "\r\n";
            }
            //fourth line
            string_to_print =
                string_to_print + "Volume: " + sink.volume().to_string().as_str() + "\r\n";
            for _n in 0..N_OF_LINES {
                execute!(std::io::stdout(), MoveUp(1), Clear(ClearType::CurrentLine));
            }
            print!("{}", string_to_print);
        }
    }
    execute!(
        std::io::stdout(),
        DisableBracketedPaste,
        DisableFocusChange,
        DisableMouseCapture
    );
    disable_raw_mode();
}

fn toggle_pause(sink: &Sink) {
    if sink.is_paused() {
        sink.play();
    } else {
        sink.pause();
    }
}
fn next_song(sink: &Sink) {
    sink.skip_one();
}

fn increase_volume(sink: &Sink) {
    if sink.volume() >= VOL_MAX {
        return;
    }
    sink.set_volume(((sink.volume() + CHANGE_VOL_BY) * 10.0).ceil() / 10.0);
}
fn decrease_volume(sink: &Sink) {
    if sink.volume() <= VOL_MIN {
        return;
    }

    sink.set_volume(((sink.volume() - CHANGE_VOL_BY) * 10.0).ceil() / 10.0);
}
// backend -- sink bs

fn add_file_to_empty_sink(sink: &Sink, file_dir: &str) {
    let file_raw;
    match File::open(DIR.to_owned() + file_dir) {
        Ok(f) => file_raw = f,
        Err(e) => {
            println!("Err, error: {:?}", e);
            exit();
            return;
        }
    }
    let file = BufReader::new(file_raw);
    let source = Decoder::new(file).unwrap();

    sink.clear();
    sink.append(source);
    sink.play();
}

fn get_file(file_dir: &str) -> BufReader<File> {
    let file_raw;
    match File::open(DIR.to_owned() + file_dir) {
        Ok(f) => file_raw = f,
        Err(e) => {
            println!("Err, error: {:?}", e);
            println!("too lazy to do error handling, sorry ");
            exit();
            panic!("");
        }
    }
    let file = BufReader::new(file_raw);
    return file;
}

fn get_file_duration(file_dir: &str) -> String {
    let file = get_file(file_dir);
    let src = Decoder::new(file).unwrap();
    let mut len_in_sec: i32 = src.total_duration().unwrap().as_secs() as i32;
    let mut hrs: i32 = 0;
    let mut mins: i32 = 0;
    let secs: i32;
    let hrs_as_str: String;
    let mins_as_str: String;
    let secs_as_str: String;
    if len_in_sec >= 3600 {
        hrs = (len_in_sec as f64 / 3600.0).floor() as i32;
        len_in_sec -= hrs * 3600;
    }
    if len_in_sec >= 60 {
        mins = (len_in_sec as f64 / 60.0).floor() as i32;
        len_in_sec -= mins * 60;
    }
    secs = len_in_sec;
    if hrs < 10 {
        hrs_as_str = "0".to_owned() + hrs.to_string().as_str();
    } else {
        hrs_as_str = hrs.to_string();
    }
    if mins < 10 {
        mins_as_str = "0".to_owned() + mins.to_string().as_str();
    } else {
        mins_as_str = mins.to_string();
    }
    if secs < 10 {
        secs_as_str = "0".to_owned() + secs.to_string().as_str();
    } else {
        secs_as_str = secs.to_string();
    }

    let str: String;
    if hrs != 0 {
        str = hrs_as_str + ":" + mins_as_str.as_str() + ":" + secs_as_str.as_str();
    } else {
        str = mins_as_str + ":" + secs_as_str.as_str();
    }
    return str;
}
fn get_current_song_played_duration(sink: &Sink) -> String {
    let mut len_in_sec: i32 = sink.get_pos().as_secs() as i32;
    let mut hrs: i32 = 0;
    let mut mins: i32 = 0;
    let secs: i32;
    let hrs_as_str: String;
    let mins_as_str: String;
    let secs_as_str: String;
    if len_in_sec >= 3600 {
        hrs = (len_in_sec as f64 / 3600.0).floor() as i32;
        len_in_sec -= hrs * 3600;
    }
    if len_in_sec >= 60 {
        mins = (len_in_sec as f64 / 60.0).floor() as i32;
        len_in_sec -= mins * 60;
    }
    secs = len_in_sec;
    if hrs < 10 {
        hrs_as_str = "0".to_owned() + hrs.to_string().as_str();
    } else {
        hrs_as_str = hrs.to_string();
    }
    if mins < 10 {
        mins_as_str = "0".to_owned() + mins.to_string().as_str();
    } else {
        mins_as_str = mins.to_string();
    }
    if secs < 10 {
        secs_as_str = "0".to_owned() + secs.to_string().as_str();
    } else {
        secs_as_str = secs.to_string();
    }
    let str: String;
    if hrs != 0 {
        str = hrs_as_str + ":" + mins_as_str.as_str() + ":" + secs_as_str.as_str();
    } else {
        str = mins_as_str + ":" + secs_as_str.as_str();
    }
    return str;
}

fn generate_random_queue() -> Vec<usize> {
    let files = list_music_files();
    let mut list: Vec<usize> = vec![];
    for i in 0..files.len() {
        list.push(i);
    }
    list.shuffle(&mut thread_rng());
    return list;
}

//backend --misc

fn exit() {
    execute!(
        std::io::stdout(),
        DisableBracketedPaste,
        DisableFocusChange,
        DisableMouseCapture
    );
    disable_raw_mode();
    process::exit(0x0100);
}
fn wait(time: u64) {
    let idk = time::Duration::from_millis(time);
    let now = time::Instant::now();

    thread::sleep(idk);

    assert!(now.elapsed() >= idk);
}
fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}
fn read_stdin() -> String {
    let mut buffer = String::new();
    let stdin = io::stdin();
    let res = stdin.read_line(&mut buffer);
    if let Err(e) = res {
        println!("Error while reading file: {:?}", e);
    }
    trim_newline(&mut buffer);
    return buffer;
}

fn list_music_files() -> Vec<String> {
    //return an alphabetically sorted list of songs
    let paths_raw = fs::read_dir(DIR).unwrap();
    let mut paths: Vec<String> = vec![];
    let mut files: Vec<String> = vec![];
    let mut file_path_temp: Vec<String> = vec![];
    for path in paths_raw {
        paths.push(path.unwrap().path().display().to_string());
    }
    for path in paths {
        for item in path.split("/") {
            file_path_temp.push(item.to_string());
        }
        // THIS SHIT TOOK 2 HOURS OF SLEEP DEPRIVED CODING, IF IT WORKS, DONT FUCKING TOUCH IT
        files.push(file_path_temp[file_path_temp.len() - 1].clone());
    }
    files.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    return files;
}
fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>());
}
