use std::str::FromStr;

use anyhow::{anyhow, Result};
use num::BigUint;

pub fn rsa_verify(
    s: &BigUint, pub_key: &BigUint,
    hash: &[u8; 32],
) -> bool {
    let e = BigUint::from_str("65537").unwrap();
    let p = s.modpow(&e, pub_key);
    let p = p.to_bytes_be();

    let mut m = [0u8; 256];
    m[256 - p.len()..].copy_from_slice(&p);

    let sha_algo_param: [u8; 19] = [
        0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 
        0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05, 
        0x00, 0x04, 0x20
    ];

    let padding_len = m.len() - 3 - 19 - 32;
    if m[0] != 0x0 || m[1] != 0x1 { return false }

    for i in 0..padding_len {
        if m[i + 2] != 0xff { return false }
    }

    if m[2 + padding_len] != 0x00 { return false }

    for i in 0..19 {
        if m[2 + padding_len + 1 + i] != sha_algo_param[i] { return false }
    }

    for i in 0..32 {
        if m[2 + padding_len + 1 + 19 + i] != hash[i] { return false }
    }

    true
}

pub fn get_rsa_pub_key(provider: &str) -> Result<String> {
    match provider {
        "google" => Ok("r6dnzQXiymQg19FInr31eQjwtm0bbRC5LHx7zaENF5PXnGy56bPOMXFClzODr07WmVBd-T73xcAwI0CX6J_a9zRPhwncErT2KLlg0X7pzQp4NIn48yYnojVNKL-kp1yxOF5ySIOHVUhuR2AUIDYzQqYUGYC-LHmIKPsv7vn-1z3BfPpvuszKlaEWju6g6r9UTHsDBDbB4WhobWRRNC7F0ipKX-n7w6j_edfBImAlZ1-FSExyt1cDEfofwpexLDVn66Mt7gsNF4KhQpEGlhZBQQKgbYiDHgUQ5Yx2BJOnrL9mi-rZJ8augIyl0lQiIoAFSnWv5xmTSfrsWFDWQXM_OQ".to_string()),
        "twitter" => Ok("r6dnzQXiymQg19FInr31eQjwtm0bbRC5LHx7zaENF5PXnGy56bPOMXFClzODr07WmVBd-T73xcAwI0CX6J_a9zRPhwncErT2KLlg0X7pzQp4NIn48yYnojVNKL-kp1yxOF5ySIOHVUhuR2AUIDYzQqYUGYC-LHmIKPsv7vn-1z3BfPpvuszKlaEWju6g6r9UTHsDBDbB4WhobWRRNC7F0ipKX-n7w6j_edfBImAlZ1-FSExyt1cDEfofwpexLDVn66Mt7gsNF4KhQpEGlhZBQQKgbYiDHgUQ5Yx2BJOnrL9mi-rZJ8augIyl0lQiIoAFSnWv5xmTSfrsWFDWQXM_OQ".to_string()),
        _ => Err(anyhow!("unrecognized provider")),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn dry_run_rsa() {    
        let rsa_sig = BigUint::from_str("27166015521685750287064830171899789431519297967327068200526003963687696216659347317736779094212876326032375924944649760206771585778103092909024744594654706678288864890801000499430246054971129440518072676833029702477408973737931913964693831642228421821166326489172152903376352031367604507095742732994611253344812562891520292463788291973539285729019102238815435155266782647328690908245946607690372534644849495733662205697837732960032720813567898672483741410294744324300408404611458008868294953357660121510817012895745326996024006347446775298357303082471522757091056219893320485806442481065207020262668955919408138704593").unwrap();
        let pub_key = BigUint::from_str("27333278531038650284292446400685983964543820405055158402397263907659995327446166369388984969315774410223081038389734916442552953312548988147687296936649645550823280957757266695625382122565413076484125874545818286099364801140117875853249691189224238587206753225612046406534868213180954324992542640955526040556053150097561640564120642863954208763490114707326811013163227280580130702236406906684353048490731840275232065153721031968704703853746667518350717957685569289022049487955447803273805415754478723962939325870164033644600353029240991739641247820015852898600430315191986948597672794286676575642204004244219381500407").unwrap();
        let hash_uint = BigUint::from_str("83814198383102558219731078260892729932246618004265700685467928187377105751529").unwrap();
    
        assert!(
            rsa_verify(
                &rsa_sig, 
                &pub_key, 
                &hash_uint.to_bytes_be()[..32].try_into().unwrap()
            )
        );
        
    }
    
}