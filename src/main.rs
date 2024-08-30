//const HASH_ALGO: &str = "sha256"; // ex: [sha256, scrypt, bolthash]
//const MAX_BLOCK_SIZE: usize = 1024 * 1024; // 1mb
//const MAX_BLOCK_COMMENT_SIZE: usize = 32;

use sha2::{Sha256, Digest};
use serde_derive::{Serialize, Deserialize};
use clap::{App, Arg, SubCommand};
use std::time::{SystemTime, UNIX_EPOCH};
use num_bigint::BigInt;
use num_traits::{One, Num};

const VERSION_INT: u64 = 1;
const INTEGER_SCALE: u64 = 100_000_000;
const MAX_SUPPLY: u64 = 21_000_000 * INTEGER_SCALE;
const REWARD_DEFAULT: u64 = 50 * INTEGER_SCALE;
const DIFFICULTY_DEFAULT: u64 = 1;

const REWARD_HALVING_INTERVAL: u64 = 210_000/2; // blocks
const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 1008; // blocks
const TARGET_BLOCK_TIME: u64 = 60 * 5; // 5 minutes per block

const TARGET_TIMESPAN: u64 = TARGET_BLOCK_TIME * DIFFICULTY_ADJUSTMENT_INTERVAL;
const MAX_ADJUSTMENT_FACTOR: u64 = 2;

macro_rules! print_serialized {
    ($data:expr) => {
        println!("{}", serde_json::to_string_pretty(&$data).unwrap_or_else(|_| "Serialization Error".to_string()));
    };
}

fn calculate_block_reward(block_height: u64) -> u64 {
    let halvings = block_height / REWARD_HALVING_INTERVAL;
    if halvings >= 64 {
        return 0;
    }
    REWARD_DEFAULT / (2u64.pow(halvings as u32))
}

fn calculate_block_difficulty(chain: &Chain, block_height: u64) -> u64 {
    if block_height < DIFFICULTY_ADJUSTMENT_INTERVAL {
        return DIFFICULTY_DEFAULT;
    }

    if block_height % DIFFICULTY_ADJUSTMENT_INTERVAL != 0 {
        let last_block = chain.get_block_by_height(block_height - 1).unwrap();
        return last_block.header.difficulty_target;
    }

    let mut start_block_index = block_height - DIFFICULTY_ADJUSTMENT_INTERVAL;

    if start_block_index == 0 {
        start_block_index = 1;
    }

    let start_block = chain.get_block_by_height(start_block_index).unwrap();
    let end_block = chain.get_block_by_height(block_height - 1).unwrap();

    let mut actual_timespan = end_block.header.timestamp - start_block.header.timestamp;

    println!("actual_timespan {}", actual_timespan);
    println!("target_timespan {}", TARGET_TIMESPAN);

    actual_timespan = std::cmp::max(actual_timespan, TARGET_TIMESPAN / MAX_ADJUSTMENT_FACTOR);
    actual_timespan = std::cmp::min(actual_timespan, TARGET_TIMESPAN * MAX_ADJUSTMENT_FACTOR);

    println!("actual_timespan (adjusted) {}", actual_timespan);

    let mut new_difficulty = end_block.header.difficulty_target * TARGET_TIMESPAN / actual_timespan;
    println!("new_difficulty {}", new_difficulty);

    new_difficulty = std::cmp::max(new_difficulty, end_block.header.difficulty_target / MAX_ADJUSTMENT_FACTOR);
    new_difficulty = std::cmp::min(new_difficulty, end_block.header.difficulty_target * MAX_ADJUSTMENT_FACTOR);

    println!("new_difficulty (adjusted) {}", new_difficulty);

    new_difficulty
}

#[derive(Debug, Serialize, Deserialize)]
struct Transaction {
    from: Option<String>,
    to: String,
    amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct BlockHeader {
    timestamp: u64,
    version: u64,
    merkle: String,
    difficulty_target: u64,
    nonce: u64,
    previous_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Block {
    id: u64,
    header: BlockHeader,
    transactions: Vec<Transaction>,
    hash: String,
}

impl Block {
    fn new(id: u64, previous_hash: String, transactions: Vec<Transaction>, difficulty: u64) -> Self {
        let mut block = Block {
            id,
            header: BlockHeader {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                version: VERSION_INT,
                merkle: String::new(), // Will be updated
                difficulty_target: difficulty,
                nonce: 0,
                previous_hash,
            },
            transactions,
            hash: String::new(),
        };

        if id != 0 {
            let block_reward = calculate_block_reward(id);
            block.transactions.push(Transaction {
                from: None,
                to: "miner_address".to_string(),
                amount: block_reward,
            });
        }

        // Calculate the Merkle root after all transactions are added
        let merkle_tree = MerkleTree::new(&block.transactions);
        block.header.merkle = merkle_tree.root_hash();
        
        block
    }

    fn create_hash(&self) -> String {
        let mut hasher = Sha256::new();
        let data = format!(
            "{}{}{}{}{}{}",
            self.id, self.header.timestamp, self.header.version, self.header.merkle, self.header.difficulty_target, self.header.nonce
        );
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    fn mine(&mut self) {
        let shift_amount = (256 - self.header.difficulty_target) as usize;
        let target = (BigInt::one() << shift_amount) - BigInt::one();

        loop {
            self.hash = self.create_hash();

            // Parse the entire hash into a BigInt.
            let hash_int = BigInt::from_str_radix(&self.hash, 16).unwrap();

            if hash_int <= target {
                break;
            }

            self.header.nonce += 1;
        }
    }
}

struct MerkleTree {
    nodes: Vec<String>,
}

impl MerkleTree {
    fn new(transactions: &[Transaction]) -> Self {
        let mut nodes = transactions
            .iter()
            .map(|tx| {
                let mut hasher = Sha256::new();
                hasher.update(serde_json::to_string(tx).unwrap());
                format!("{:x}", hasher.finalize())
            })
            .collect::<Vec<_>>();

        let mut layer = 0;
        while nodes.len() > 1 {
            if nodes.len() % 2 != 0 {
                let last = nodes.last().unwrap().clone();
                nodes.push(last);
            }

            let mut parent_nodes = Vec::new();
            for i in (0..nodes.len()).step_by(2) {
                let left = &nodes[i];
                let right = &nodes[i + 1];

                let mut hasher = Sha256::new();
                hasher.update(format!("{}{}", left, right));
                let parent = format!("{:x}", hasher.finalize());
                parent_nodes.push(parent);
            }
            nodes = parent_nodes;
            layer += 1;
        }

        if nodes.is_empty() {
            nodes.push("0".repeat(64));
        }

        MerkleTree { nodes }
    }

    fn root_hash(&self) -> String {
        self.nodes[0].clone()
    }
}

struct Chain {
    blocks: Vec<Block>,
}

impl Chain {
    fn new() -> Self {
        let mut chain = Chain { blocks: vec![] };

        // Create the genesis block with a fixed timestamp
        let mut genesis_block = Block {
            id: 0,
            header: BlockHeader {
                timestamp: 0, // Fixed timestamp for the genesis block
                version: 1,
                merkle: String::new(), // Will be updated
                difficulty_target: 1,
                nonce: 0,
                previous_hash: "0".repeat(64),
            },
            transactions: vec![],
            hash: String::new(),
        };

        // Calculate the Merkle root for the genesis block
        let merkle_tree = MerkleTree::new(&genesis_block.transactions);
        genesis_block.header.merkle = merkle_tree.root_hash();

        // Mine the genesis block
        genesis_block.mine();

        chain.blocks.push(genesis_block);

        chain
    }

    fn add_block(&mut self, mut block: Block) -> Result<(), String> {
        if let Some(last_block) = self.blocks.last() {
            if block.header.previous_hash != last_block.hash {
                return Err(format!("Invalid previous hash. Expected: {}, Found: {}", last_block.hash, block.header.previous_hash));
            }
        }

        let merkle_tree = MerkleTree::new(&block.transactions);
        if merkle_tree.root_hash() != block.header.merkle {
            return Err(format!("Invalid merkle hash. Expected: {}, Found: {}", merkle_tree.root_hash(), block.header.merkle));
        }

        block.mine();
        self.blocks.push(block);

        Ok(())
    }

    fn drop_chain(&mut self) {
        self.blocks.clear();
    }

    fn get_last_block(&self) -> Option<&Block> {
        self.blocks.last()
    }

    fn get_block_by_height(&self, height: u64) -> Option<&Block> {
        self.blocks.iter().find(|block| block.id == height)
    }
}

fn main() {
    let mut chain = Chain::new();

    let matches = App::new("Blockchain")
        .subcommand(SubCommand::with_name("drop").about("Drop all blocks from the chain"))
        .subcommand(
            SubCommand::with_name("mine")
                .about("Mine new blocks")
                .arg(
                    Arg::with_name("count")
                        .short("c")
                        .long("count")
                        .value_name("COUNT")
                        .help("Number of blocks to mine")
                        .takes_value(true)
                        .default_value("1"),
                )
                .arg(
                    Arg::with_name("dump")
                        .long("dump")
                        .help("Dump all serialized objects in the chain"),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("drop", Some(_)) => {
            chain.drop_chain();
            println!("All blocks dropped from the chain.");
        }
        ("mine", Some(sub_matches)) => {
            let count: u64 = sub_matches.value_of("count").unwrap_or("1").parse().expect("Invalid count");

            for _ in 0..count {
                let last_block = chain.get_last_block();
                let last_block_id = last_block.map(|block| block.id).unwrap_or(0);
                let last_block_hash = last_block.map(|block| block.hash.clone()).unwrap_or_else(|| "0".repeat(64));

                let difficulty = calculate_block_difficulty(&chain, last_block_id + 1);

                let mut block = Block::new(
                    last_block_id + 1,
                    last_block_hash,
                    vec![], // Empty transactions for now
                    difficulty,
                );

                match chain.add_block(block) {
                    Ok(_) => {
                        let last_block = chain.get_last_block().expect("Last block should exist");
                        print_serialized!(last_block);
                    }
                    Err(err) => {
                        println!("Error adding block: {}", err);
                    }
                }
            }

            if sub_matches.is_present("dump") {
                println!("Dumping serialized objects in the chain:");
                for block in &chain.blocks {
                    print_serialized!(block);
                }
            }
        }
        _ => {
            println!("Invalid command. Usage: cargo run -- [drop|mine --count <n> --dump]");
        }
    }
}

