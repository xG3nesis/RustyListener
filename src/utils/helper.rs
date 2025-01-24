use std::{time::Duration, process::Command};
use bluer::{Address, Adapter, AdapterEvent, Device};
use regex::Regex;
use tokio::select;
use futures::{pin_mut, StreamExt};

// Function "assert_addr" used to assert the bluetooth target address is in the right format.
pub fn assert_addr(val: &str) -> Result<Address, String> {
    let re = Regex::new(r"^([0-9A-Fa-f]{2}(:[0-9A-Fa-f]{2}){5})$").unwrap();
    if re.is_match(val) {
        Ok(val.parse().unwrap())
    } else {
        Err(String::from(
            "Invalid Bluetooth address format. Expected format in hexadecimal : XX:XX:XX:XX:XX:XX !",
        ))
    }
}

// Function "assert_sink" used to assert that audio card output exists or is available.
pub fn assert_sink(val: &str) -> Result<String, String> {
    let reserve = val.to_string();

    let list_cards = Command::new("pactl")
            .args(["list", "cards"])
            .output()
            .expect("Failed to check available audio cards !");
    
    if !String::from_utf8_lossy(&list_cards.stdout).contains(val) {
        return Err(String::from("Invalid specified sink. Specified sink not available or doesn't exists on device.
                                Please run 'pactl list cards' to check by yourself ! and specify valid available sink !"))
    } else {
        return Ok(reserve)
    }

}

// Function "find_device" used to search for devices in discoverable mode !
pub async fn find_device(adapter: &Adapter, address: Address) -> Result<Device, String> {
    let mut disco = adapter.discover_devices().await.unwrap();
    let timeout = tokio::time::sleep(Duration::from_secs(20));
    pin_mut!(timeout);

    loop {
        select! {
            Some(evt) = disco.next() => {
                if let AdapterEvent::DeviceAdded(addr) = evt {
                    if addr == address {
                        return Ok(adapter.device(addr).unwrap());
                    }
                }
            }
            _ = &mut timeout => {
                return Err("device not found".into());
            }
        }
    }
}

// Function "nondisc_pair_attempt" used to seek for devices in non discoverable and attempt to reach them.
pub fn nondisc_pair_attempt(address: String) -> bool {
    let no_input_no_output: &str = "3";

    let btmgmt = Command::new("btmgmt")
        .args(["pair", "-c", no_input_no_output, &address])
        .output()
        .expect("Failed to launch pairing!");

    let stdout = std::str::from_utf8(&btmgmt.stdout).unwrap_or("");
    let stderr = std::str::from_utf8(&btmgmt.stderr).unwrap_or("");

    let combined_output = format!("{}{}", stdout, stderr);

    if combined_output.contains("failed") || combined_output.contains("status 0x05 (Authentication Failed)") {
        return false;
    }

    true
}

#[cfg(test)]
mod check_addr_tests {
    use super::*;

    #[test]
    fn check_nice_addr_format(){
        let addr = Address::from([0xA1, 0xB1, 0xC1, 0xD1, 0xE1, 0xF1]);
        let result = assert_addr("A1:B1:C1:D1:E1:F1").unwrap();
        assert_eq!(result, addr);
    }

    #[test]
    #[should_panic]
    fn wrong_format_size_upper(){
        let _result = assert_addr("A1:B1:C1:D1:E1:F1:G1").unwrap();
    }

    #[test]
    #[should_panic]
    fn wrong_format_size_lower(){
        let _result = assert_addr("A1:B1:C1:D1").unwrap();
    }
}

#[cfg(test)]
mod check_audio_cards {
    use super::*;

    #[test]
    fn check_card_found(){
        let real_card = "alsa_card.pci-0000_00_05.0";
        let result = assert_sink(real_card).unwrap();
        assert_eq!(result, real_card.to_string())
    }

    #[test]
    #[should_panic]
    fn check_card_not_found(){
        let real_card = "alsa_card.pci-10000000";
        let result = assert_sink(real_card).unwrap();
        assert_eq!(result, real_card.to_string())
    }
}