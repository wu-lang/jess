use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Write};

use std::path::{Path, PathBuf};
use std::cell::RefCell;

extern crate colored;
use colored::Colorize;

extern crate toml;
use toml::Value;

extern crate git2;

use git2::build::{RepoBuilder, CheckoutBuilder};
use git2::{RemoteCallbacks, Progress, FetchOptions};



const HELP: &'static str = "\
The Jess Package Manager

Usage:
    jess           # Display this
    jess <command> # Read further below

Commands:
    new <name>     # Creates a new project at optional path
    build          # Builds the current project
    run            # Runs the current project
";



pub struct State {
  pub progress: Option<Progress<'static>>,
  pub total:    usize,
  pub current:  usize,
  pub path:     Option<PathBuf>,
  pub newline:  bool
}



fn new(name: Option<&str>) {
  if let Some(name) = name {
    if Path::new(name).exists() {
      wrong(&format!("path '{}' already exists", name));
    } else {
      fs::create_dir_all(format!("{}/src", name)).unwrap();

      let mut init = File::create(&format!("{}/init.wu", name)).unwrap();
      init.write_all(b"# exposing things for library use\nimport src\n").unwrap();

      let mut jess_toml = File::create(&format!("{}/jess.toml", name)).unwrap();
      jess_toml.write_all(b"[dependencies]\n").unwrap();

      File::create(&format!("{}/src/init.wu", name)).unwrap();
    }
  } else {
    let mut jess_toml = File::create("jess.toml").unwrap();
    jess_toml.write_all(b"[dependencies]").unwrap();

    let mut jess_init = File::create("src/init.wu").unwrap();
  }
}


fn get() {
  if Path::new("jess.toml").exists() {
    let mut config = File::open("jess.toml").unwrap();
    
    let mut contents = String::new();
    config.read_to_string(&mut contents).unwrap();

    match toml::from_str::<Value>(&contents) {
      Ok(value) => match value.get("dependencies") {
        Some(depends) => match *depends {
          Value::Table(ref t) => {
            let mut modules = Vec::new();

            for member in t {
              if !Path::new("src/lib/").exists() {
                fs::create_dir("src/lib/").unwrap();
              }

              if let Value::String(ref url) = *member.1 {
                let path = &format!("src/lib/{}", member.0);

                if Path::new(path).exists() {
                  fs::remove_dir_all(path).unwrap()
                }

                println!("{}", format!("{} {}", "Downloading".green().bold(), member.0.white().bold()));
                clone(&format!("https://github.com/{}", url), path);

                modules.push(format!("import {}", member.0))
              } else {
                wrong("Expected string URL value")
              }
            }
            
            if modules.len() > 0 {
              let mut init = File::create("src/lib/init.wu").unwrap();
              init.write_all(modules.join("\n").as_bytes()).unwrap();
            }
          },

          _ => wrong(r#"Expected key e.g. `a = "b"`"#),
        }
        _ => (),
      },

      Err(_)  => wrong("Something went wrong in 'jess.toml'"),
    }

  } else {
      wrong("Couldn't find 'jess.toml'");
  }
}



fn print(state: &mut State) {
  let stats       = state.progress.as_ref().unwrap();
  let network_pct = (100 * stats.received_objects()) / stats.total_objects();
  let index_pct   = (100 * stats.indexed_objects()) / stats.total_objects();

  let co_pct      = if state.total > 0 {
    (100 * state.current) / state.total
  } else {
    0
  };

  let kbytes = stats.received_bytes() / 1024;

  if stats.received_objects() == stats.total_objects() {
    if !state.newline {
      println!("");
      state.newline = true
    }

    let deltas = format!("{}/{}\r", stats.indexed_deltas(), stats.total_deltas());
    print!("\t{} {}", "progress:".bold(), deltas.yellow().bold())
  } else {
    let progress = format!("{} {:3}% ({:4} kb, {:5}/{:5})  /  {} {:3}% ({:5}/{:5})  \
      /  {} {:3}% ({:4}/{:4}) {}\r",
      "net".white().bold(),
      network_pct, kbytes, stats.received_objects(),
      stats.total_objects(),

      "idx".white().bold(),
      index_pct, stats.indexed_objects(), stats.total_objects(),

      "chk".white().bold(),
      co_pct, state.current, state.total,
      state.path.as_ref()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default());
                
    print!("\t{}", progress.green().bold())
  }

  io::stdout().flush().unwrap();
}



fn clone(url: &str, path: &str) {
  let state = RefCell::new(State {
    progress: None,
    total: 0,
    current: 0,
    path: None,
    newline: false,
  });

  let mut cb = RemoteCallbacks::new();

  cb.transfer_progress(|stats| {
    let mut state = state.borrow_mut();
    state.progress = Some(stats.to_owned());
    print(&mut *state);
    true
  });

  let mut co = CheckoutBuilder::new();

  co.progress(|path, cur, total| {
    let mut state = state.borrow_mut();
    state.path = path.map(|p| p.to_path_buf());
    state.current = cur;
    state.total = total;
    print(&mut *state);
  });

  let mut fo = FetchOptions::new();

  fo.remote_callbacks(cb);

  match RepoBuilder::new().fetch_options(fo).with_checkout(co).clone(url, Path::new(path)) {
    Ok(_)  => (),
    Err(_) => wrong(&format!("failed to download '{}'", url))
  }

  println!()
}



fn wrong(message: &str) {
  println!("{} {}", "wrong:".red().bold(), message)
}


fn main() {
  let args = env::args().collect::<Vec<String>>();

  if args.len() == 1 {
    println!("{}", HELP);
  } else {
    match args[1].as_str() {
      "new" => if args.len() > 2 {
        new(Some(&args[2]))
      } else {
        new(None)
      },

      "get" => get(),

      _ => println!("{}", HELP),
    }
  }
}