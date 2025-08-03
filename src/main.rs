// use clap::Parser;

// use templatr::prompt::prompt;

// /// Templatr rust rewrite (implementation not based on the old one)
// #[derive(Parser, Debug)]
// #[clap(author, version, about, long_about = None)]
// struct Args {
//     /// Link to the git repo, sinonymous with the git clone link
//     #[clap(short, long)]
//     git: String,

//     #[clap(short, long)]
//     branch: Option<String>,

//     /// Destination where template will be copied to. FILES WILL BE OVERWRITTEN
//     dest: String,
// }

// fn main() -> color_eyre::Result<()> {
//     color_eyre::install()?;
//     let args = Args::parse();

//     let git = args.git;
//     let dest = args.dest;

//     prompt(&git, &dest, args.branch.as_deref())
// }

// For some reason Kotlin is too slow on these machines

// package compete1

// import kotlin.math.max

// @OptIn(kotlin.ExperimentalStdlibApi::class)
// fun main(args: Array<String>) {
//     val line = readln().split(" ").map { it.toInt() }
//     val commercialBreaks = line[0] // N <= 100_000
//     val pricePerCommercial = line[1] // P <= 1_000

//     val listening = readln().split(" ").map { it.toInt() }

//     var profit = calculateProfit(pricePerCommercial, listening)

//     // 0..<listening.size
//     for (i in listening.indices) {
//         for (j in i..<listening.size) {
//             val subList = listening.subList(i, j)

//             profit = max(profit, calculateProfit(pricePerCommercial, subList))
//         }
//     }

//     println(profit)
// }

// private fun calculateProfit(
//     pricePerCommercial: Int,
//     listening: List<Int>
// ): Int {
//     val cost = listening.size * pricePerCommercial
//     val revenue = listening.sum()
//     val profit = revenue - cost

//     return profit
// }

use std::io;

fn main() {
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();

    let mapped_line: Vec<i32> = line
        .trim()
        .replace("\r\n", "")
        .replace('\n', "")
        .split(' ')
        .map(|x| x.parse::<i32>().unwrap())
        .collect();
    let commercial_breaks = mapped_line[0]; // N <= 100_000
    let price_per_commercial = mapped_line[1]; // P <= 1_000

    let mut listening_line = String::new();
    io::stdin().read_line(&mut listening_line).unwrap();
    let listening: Vec<i32> = listening_line
        .trim()
        .replace("\r\n", "")
        .replace('\n', "")
        .split(' ')
        .map(|x| x.parse::<i32>().unwrap())
        .filter(|x| *x != 0)
        .collect();

    

    let mut profit = 0; // calculate_profit(price_per_commercial, &listening);

    let highest = listening.iter().max().unwrap();
    let highest_index = listening.iter().position(|x| x == highest).unwrap();

    // for i in 0..listening.len() - 1 {
    //     for j in i + 1..listening.len() {
    //         let sublist = &listening[i..j];
    //         profit = calculate_profit(price_per_commercial, sublist).max(profit);

    //     }
    // }

    // try implement binary search
    
    for i in 0..listening.len() / 2 {
        for j in 0..listening.len() / 2 {
            let sublist = &listening[highest_index - i..highest_index + j];
            let new_profit = calculate_profit(price_per_commercial, sublist).max(profit);

            if new_profit < profit { break; }

            profit = new_profit;
        }
    }

    //     for i in 0..range {
    //     let start_index = highest_index - i;
    //     let mut people = listening[start_index];
    //     println!("Start {people}");

    //     for j in 1..listening.len() - start_index {
    //         let end_index = start_index + j;
    //         let commercials = (i + j - 2) as i32;
    //         // if highest_index + j != highest_index - i {
    //         //     println!("Add {}", listening[highest_index + j]);
    //         //     people += listening[highest_index + j];
    //         // }

    //             println!("Add {}", listening[end_index]);
    //             people += listening[end_index];
            

    //         let new_profit = calculate_profit(price_per_commercial, people, commercials);

    //         // if new_profit < profit { break; }

    //         profit = new_profit.max(profit);
    //         println!("{i} {j} {commercials} {people} {profit} {new_profit}")
    //     }
    // }


    println!("{}", profit);
}

fn calculate_profit(price_per_commercial: i32, listening: &[i32]) -> i32 {
    let cost = listening.len() as i32 * price_per_commercial;
    let revenue = listening.iter().sum::<i32>();

    revenue - cost
}
