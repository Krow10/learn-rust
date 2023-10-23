use rand::Rng;
use std::io;

fn main() {
    loop {
        let rng_iter = rand::thread_rng().sample_iter(rand::distributions::Standard);
        /*
            Real-life slot machines stores the result before user starts the spin. That way, you can program the display according
            to the pre-calculated result with certain features such as showing a particular sound, visual etc. Note that certain
            regulations (US) do not allow to program such things as "near-miss" (that could incitivise player to play more thinking it
            "almost" won) based on the pre-calculated result.
        */
        let spin_result: Vec<u32> = rng_iter.take(3).collect();
        println!("Enter any input to start a spin!");

        let mut user_input = String::new();

        io::stdin()
            .read_line(&mut user_input)
            .expect("Failed to read user input");

        println!("{:?}", spin_result);
    }
}