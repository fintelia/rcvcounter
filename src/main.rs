use std::{
    collections::HashMap,
    iter,
    sync::{Arc, Mutex},
};

use anyhow::Error;

use rand::seq::SliceRandom;

fn main() -> Result<(), Error> {
    colored::control::set_override(true);

    let ballot_data = rcvcounter::parse_ballots(
        include_bytes!("../ballots2021.zip"),
        rcvcounter::Race::School,
    )?;

    let reverse_candidate_mapping = iter::once("Exhausted")
        .chain(ballot_data.candidates.iter().map(|x| x.as_str()))
        .collect::<Vec<_>>();
    let ballots = ballot_data
        .ballots
        .iter()
        .cloned()
        .flatten()
        .collect::<Vec<_>>();

    let ballot_ids = ballot_data
        .ballot_ids
        .iter()
        .cloned()
        .flatten()
        .collect::<Vec<_>>();

    const THREADS: usize = 20;
    const ITERS: usize = 5000;

    let normal = rcvcounter::run(
        ballot_data.seats,
        &ballots,
        &reverse_candidate_mapping,
        &ballot_ids,
    );

    let councils = Arc::new(Mutex::new(HashMap::new()));
    let mut joins = Vec::new();
    for _ in 0..THREADS {
        let normal = normal.clone();
        let mut ballots = ballot_data.ballots.clone();
        let councils = councils.clone();
        joins.push(std::thread::spawn(move || {
            for _ in 0..ITERS {
                for b in &mut ballots {
                    b.shuffle(&mut rand::thread_rng());
                }
                let ballots = ballots.iter().cloned().flatten().collect::<Vec<_>>();
                let result = rcvcounter::run(ballot_data.seats, &ballots, &[], &[]);
                if result != normal {
                    *councils.lock().unwrap().entry(result).or_insert(0) += 1;
                }
            }
        }));
    }
    for j in joins {
        j.join().unwrap();
    }

    println!();
    let mut sum = 0;
    for (council, n) in councils.lock().unwrap().iter() {
        println!(
            "{:.4}%: {}",
            100.0 * (*n as f32) / (ITERS * THREADS) as f32,
            council
                .iter()
                .map(|&x| reverse_candidate_mapping[x as usize])
                .collect::<Vec<_>>()
                .join(", ")
        );
        sum += n;
    }
    println!(
        "{:.4}%: {}",
        100.0 * ((ITERS * THREADS - sum) as f32) / (ITERS * THREADS) as f32,
        normal
            .iter()
            .map(|&x| reverse_candidate_mapping[x as usize])
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(())
}
