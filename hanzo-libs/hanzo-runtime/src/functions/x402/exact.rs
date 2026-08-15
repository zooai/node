//! The x402 `exact` scheme on EVM.
//!
//! A payment is an EIP-3009 `TransferWithAuthorization`: the payer signs, under
//! EIP-712, an authorization for the asset contract to move an exact amount to
//! the payee within a time window. The signature plus its authorization travel
//! base64-encoded in the payment header; whoever settles submits them on chain.

use std::borrow::Cow;
use std::time::{SystemTime, UNIX_EPOCH};

use alloy_primitives::{Address, Signature, B256, U256};
use alloy_sol_types::{Eip712Domain, SolStruct};
use base64::alphabet;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::Engine as _;
use hanzo_messages::schemas::x402_types::{
    PaymentAuthorization, PaymentPayload, PaymentPayloadData, PaymentRequirements,
};
use k256::ecdsa::SigningKey;
use rand::RngCore;

use super::network;
use crate::RunError;

alloy_sol_types::sol! {
    struct TransferWithAuthorization {
        address from;
        address to;
        uint256 value;
        uint256 validAfter;
        uint256 validBefore;
        bytes32 nonce;
    }
}

/// How far back the authorization is dated, so that a payer whose clock runs
/// ahead of the facilitator's still produces an already-valid authorization.
const BACKDATE: u64 = 600;

/// Standard base64, and forgiving about padding when reading.
const CODEC: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// Read an atomic amount.
///
/// Amounts are whole numbers, but a payee running a floating-point
/// implementation can quote one as `"7900.000000000001"` or
/// `"15699.999999999998"`. Both mean the nearest whole unit.
pub fn amount(text: &str) -> Result<U256, RunError> {
    let text = text.trim();
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(RunError::SerializeParamsError(format!("amount {text} is not a number")));
    }
    let mut value = U256::from_str_radix(whole, 10)
        .map_err(|e| RunError::SerializeParamsError(format!("amount {text} is not a number: {e}")))?;
    if fraction.starts_with(['5', '6', '7', '8', '9']) {
        value += U256::from(1u8);
    }
    Ok(value)
}

fn address(text: &str, what: &str) -> Result<Address, RunError> {
    text.parse::<Address>()
        .map_err(|e| RunError::SerializeParamsError(format!("{what} is not an address: {e}")))
}

/// The EIP-712 domain the asset contract verifies against.
///
/// `name` and `version` are whatever the payee advertised in `extra`, because
/// they must match what the deployed contract returns. Only when the payee
/// says nothing do we fall back to the known deployment for the network.
fn domain(requirements: &PaymentRequirements) -> Result<Eip712Domain, RunError> {
    let advertised = |field: &str| -> Option<String> {
        requirements
            .extra
            .as_ref()?
            .get(field)?
            .as_str()
            .map(|text| text.to_string())
    };
    let fallback = network::asset(&requirements.network).eip712;
    Ok(Eip712Domain::new(
        Some(Cow::Owned(advertised("name").unwrap_or(fallback.name))),
        Some(Cow::Owned(advertised("version").unwrap_or(fallback.version))),
        Some(U256::from(network::chain(&requirements.network))),
        Some(address(&requirements.asset, "asset")?),
        None,
    ))
}

/// Sign an authorization to pay `requirements` from `key`.
pub fn sign(key: &SigningKey, requirements: &PaymentRequirements, x402_version: u32) -> Result<PaymentPayload, RunError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| RunError::CodeExecutionError(format!("system clock is before the epoch: {e}")))?
        .as_secs();

    let mut nonce = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce);

    let authorization = TransferWithAuthorization {
        from: Address::from_private_key(key),
        to: address(&requirements.pay_to, "payTo")?,
        value: amount(&requirements.max_amount_required)?,
        validAfter: U256::from(now.saturating_sub(BACKDATE)),
        validBefore: U256::from(now + requirements.max_timeout_seconds),
        nonce: B256::from(nonce),
    };

    let digest = authorization.eip712_signing_hash(&domain(requirements)?);
    let (signature, recovery) = key
        .sign_prehash_recoverable(digest.as_slice())
        .map_err(|e| RunError::CodeExecutionError(format!("could not sign the authorization: {e}")))?;
    let signature = Signature::from_signature_and_parity(signature, recovery.is_y_odd());

    Ok(PaymentPayload {
        scheme: requirements.scheme.clone(),
        network: requirements.network.clone(),
        x402_version,
        payload: PaymentPayloadData {
            signature: format!("0x{}", hex::encode(signature.as_bytes())),
            authorization: PaymentAuthorization {
                from: authorization.from.to_checksum(None),
                to: authorization.to.to_checksum(None),
                value: authorization.value.to_string(),
                valid_after: authorization.validAfter.to_string(),
                valid_before: authorization.validBefore.to_string(),
                nonce: format!("0x{}", hex::encode(nonce)),
            },
        },
    })
}

/// Encode a payment for the `X-PAYMENT` header.
pub fn encode(payment: &PaymentPayload) -> Result<String, RunError> {
    let json = serde_json::to_string(payment)
        .map_err(|e| RunError::SerializeParamsError(format!("could not encode the payment: {e}")))?;
    Ok(CODEC.encode(json))
}

/// Encode a settlement reply for the payment-response header.
pub fn receipt(settlement: &serde_json::Value) -> Result<String, RunError> {
    let json = serde_json::to_string(settlement)
        .map_err(|e| RunError::SerializeParamsError(format!("could not encode the settlement: {e}")))?;
    Ok(CODEC.encode(json))
}

/// Decode an `X-PAYMENT` header.
pub fn decode(header: &str) -> Result<PaymentPayload, RunError> {
    let json = CODEC
        .decode(header.trim())
        .map_err(|e| RunError::ParseOutputError(format!("payment header is not base64: {e}")))?;
    serde_json::from_slice(&json)
        .map_err(|e| RunError::ParseOutputError(format!("payment header is not a payment payload: {e}")))
}

/// The advertised requirement a payment answers: same scheme, same network.
pub fn matching<'a>(accepts: &'a [PaymentRequirements], payment: &PaymentPayload) -> Option<&'a PaymentRequirements> {
    accepts
        .iter()
        .find(|candidate| candidate.scheme == payment.scheme && candidate.network == payment.network)
}

/// The address that signed an authorization, recovered from it.
///
/// Settlement is what a signature is finally judged by, so nothing here
/// recovers in anger; the tests use it to prove a signed authorization really
/// does come back to its signer under the domain it was signed for.
#[cfg(test)]
pub fn payer(payment: &PaymentPayload, requirements: &PaymentRequirements) -> Result<Address, RunError> {
    let authorization = &payment.payload.authorization;
    let restated = TransferWithAuthorization {
        from: address(&authorization.from, "from")?,
        to: address(&authorization.to, "to")?,
        value: amount(&authorization.value)?,
        validAfter: amount(&authorization.valid_after)?,
        validBefore: amount(&authorization.valid_before)?,
        nonce: authorization
            .nonce
            .parse::<B256>()
            .map_err(|e| RunError::SerializeParamsError(format!("nonce is not 32 bytes: {e}")))?,
    };
    let digest = restated.eip712_signing_hash(&domain(requirements)?);

    let bytes = hex::decode(payment.payload.signature.strip_prefix("0x").unwrap_or(&payment.payload.signature))
        .map_err(|e| RunError::SerializeParamsError(format!("signature is not hex: {e}")))?;
    let signature = Signature::try_from(bytes.as_slice())
        .map_err(|e| RunError::SerializeParamsError(format!("signature is malformed: {e}")))?;
    signature
        .recover_address_from_prehash(&digest)
        .map_err(|e| RunError::CodeExecutionError(format!("could not recover the payer: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256};
    use hanzo_messages::schemas::x402_types::Network;
    use serde_json::json;

    fn key() -> SigningKey {
        // Hardhat's published account 0.
        SigningKey::from_slice(
            &hex::decode("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80").unwrap(),
        )
        .unwrap()
    }

    fn requirements() -> PaymentRequirements {
        PaymentRequirements {
            scheme: "exact".to_string(),
            description: String::new(),
            network: Network::BaseSepolia,
            max_amount_required: "1000".to_string(),
            resource: "https://hanzo.ai".to_string(),
            mime_type: String::new(),
            pay_to: "0x9858EfFD232B4033E47d90003D41EC34EcaEda94".to_string(),
            max_timeout_seconds: 300,
            asset: "0x036CbD53842c5426634e7929541eC2318f3dCF7e".to_string(),
            output_schema: Some(json!({})),
            extra: Some(json!({ "name": "USDC", "version": "2" })),
        }
    }

    /// EIP-3009 publishes this type hash. Everything downstream — the digest,
    /// the signature, the on-chain check — is wrong if it does not match.
    #[test]
    fn transfer_authorization_type_hash_is_the_published_one() {
        assert_eq!(
            TransferWithAuthorization::eip712_encode_type(),
            "TransferWithAuthorization(address from,address to,uint256 value,uint256 validAfter,uint256 validBefore,bytes32 nonce)"
        );
        let empty = TransferWithAuthorization {
            from: Address::ZERO,
            to: Address::ZERO,
            value: U256::ZERO,
            validAfter: U256::ZERO,
            validBefore: U256::ZERO,
            nonce: B256::ZERO,
        };
        assert_eq!(
            TransferWithAuthorization::eip712_type_hash(&empty),
            b256!("7c7c6cdb67a18743f49ec6fa9b35f50d52ed05cbed4cc592e13b44501c1a2267")
        );
    }

    alloy_sol_types::sol! {
        struct Person { string name; address wallet; }
        struct Mail { Person from; Person to; string contents; }
    }

    /// The EIP-712 specification's own worked example, which publishes the
    /// domain separator, the signing hash and the signature. This pins the
    /// whole typed-data path that `sign` relies on.
    #[test]
    fn eip712_matches_the_specification_example() {
        let domain = Eip712Domain::new(
            Some(Cow::Borrowed("Ether Mail")),
            Some(Cow::Borrowed("1")),
            Some(U256::from(1u64)),
            Some(address!("CcCCccccCCCCcCCCCCCcCcCccCcCCCcCcccccccC")),
            None,
        );
        assert_eq!(
            domain.separator(),
            b256!("f2cee375fa42b42143804025fc449deafd50cc031ca257e0b194a650a912090f")
        );

        let mail = Mail {
            from: Person {
                name: "Cow".into(),
                wallet: address!("CD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826"),
            },
            to: Person {
                name: "Bob".into(),
                wallet: Address::from([0xbbu8; 20]),
            },
            contents: "Hello, Bob!".into(),
        };
        let digest = mail.eip712_signing_hash(&domain);
        assert_eq!(
            digest,
            b256!("be609aee343fb3c4b28e1df9e632fca64fcfaede20f02e86244efddf30957bd2")
        );

        // The example's private key produces the example's signature, which
        // also pins our r‖s‖v byte order and the v = 27 + parity convention.
        let signer = SigningKey::from_slice(
            &hex::decode("c85ef7d79691fe79573b1a7064c19c1a9819ebdbd1faaab1a8ec92344438aaf4").unwrap(),
        )
        .unwrap();
        let (signature, recovery) = signer.sign_prehash_recoverable(digest.as_slice()).unwrap();
        let signature = Signature::from_signature_and_parity(signature, recovery.is_y_odd());
        assert_eq!(
            format!("0x{}", hex::encode(signature.as_bytes())),
            "0x4355c47d63924e8a72e509b65029052eb6c299d53a04e167c5775fd466751c9d\
             07299936d304c153f6443dfa05f40ff007d72911b6f72307f996231605b915621c"
                .replace(char::is_whitespace, "")
        );
    }

    #[test]
    fn a_signed_payment_recovers_to_its_signer() {
        let requirements = requirements();
        let payment = sign(&key(), &requirements, 1).unwrap();

        assert_eq!(payment.scheme, "exact");
        assert_eq!(payment.network, Network::BaseSepolia);
        assert_eq!(payment.x402_version, 1);
        assert_eq!(
            payment.payload.authorization.from,
            "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
        );
        assert_eq!(payment.payload.authorization.to, requirements.pay_to);
        assert_eq!(payment.payload.authorization.value, "1000");
        assert_eq!(payer(&payment, &requirements).unwrap(), Address::from_private_key(&key()));
    }

    #[test]
    fn a_payment_signed_for_another_payee_does_not_recover() {
        let payment = sign(&key(), &requirements(), 1).unwrap();
        let mut elsewhere = requirements();
        elsewhere.asset = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string();
        // A different verifying contract is a different domain, so the same
        // signature must not recover the payer.
        assert_ne!(payer(&payment, &elsewhere).unwrap(), Address::from_private_key(&key()));
    }

    #[test]
    fn authorization_window_brackets_now() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let payment = sign(&key(), &requirements(), 1).unwrap();
        let after: u64 = payment.payload.authorization.valid_after.parse().unwrap();
        let before: u64 = payment.payload.authorization.valid_before.parse().unwrap();
        assert!(after <= now - BACKDATE + 2 && after >= now - BACKDATE - 2);
        assert!(before >= now + 300 - 2);
        assert!(after < before);
    }

    #[test]
    fn nonces_are_thirty_two_fresh_bytes() {
        let a = sign(&key(), &requirements(), 1).unwrap().payload.authorization.nonce;
        let b = sign(&key(), &requirements(), 1).unwrap().payload.authorization.nonce;
        assert_eq!(a.len(), 66);
        assert!(a.starts_with("0x"));
        assert_ne!(a, b);
    }

    #[test]
    fn signature_is_sixty_five_bytes() {
        let payment = sign(&key(), &requirements(), 1).unwrap();
        assert_eq!(payment.payload.signature.len(), 132);
        assert!(payment.payload.signature.starts_with("0x"));
    }

    #[test]
    fn header_is_standard_padded_base64() {
        // Standard alphabet, padded — not base64url.
        assert_eq!(CODEC.encode("x402"), "eDQwMg==");
        let payment = sign(&key(), &requirements(), 1).unwrap();
        let header = encode(&payment).unwrap();
        assert!(header.bytes().all(|b| b.is_ascii_alphanumeric() || b"+/=".contains(&b)));

        let read = decode(&header).unwrap();
        assert_eq!(read.payload.signature, payment.payload.signature);
        assert_eq!(read.payload.authorization.nonce, payment.payload.authorization.nonce);
        assert_eq!(read.x402_version, payment.x402_version);
        assert_eq!(read.network, payment.network);

        // Padding is optional on the way in.
        assert!(decode(header.trim_end_matches('=')).is_ok());
        assert!(decode("not base64!").is_err());
    }

    #[test]
    fn header_carries_the_wire_field_names() {
        let payment = sign(&key(), &requirements(), 1).unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&CODEC.decode(encode(&payment).unwrap()).unwrap()).unwrap();
        assert_eq!(json["x402Version"], 1);
        assert_eq!(json["scheme"], "exact");
        assert_eq!(json["network"], "base-sepolia");
        assert!(json["payload"]["signature"].is_string());
        for field in ["from", "to", "value", "validAfter", "validBefore", "nonce"] {
            assert!(
                json["payload"]["authorization"][field].is_string(),
                "authorization.{field} missing"
            );
        }
    }

    #[test]
    fn the_payee_advertised_domain_wins_over_the_default() {
        let mut requirements = requirements();
        requirements.extra = Some(json!({ "name": "Some Other Coin", "version": "7" }));
        let built = domain(&requirements).unwrap();
        assert_eq!(built.name.as_deref(), Some("Some Other Coin"));
        assert_eq!(built.version.as_deref(), Some("7"));

        requirements.extra = None;
        let built = domain(&requirements).unwrap();
        assert_eq!(built.name.as_deref(), Some("USDC"));
        assert_eq!(built.version.as_deref(), Some("2"));
        assert_eq!(built.chain_id, Some(U256::from(84532u64)));
    }

    #[test]
    fn amounts_round_to_the_nearest_whole_unit() {
        assert_eq!(amount("1000").unwrap(), U256::from(1000u64));
        assert_eq!(amount("0").unwrap(), U256::ZERO);
        // What a floating-point payee quotes.
        assert_eq!(amount("7900.000000000001").unwrap(), U256::from(7900u64));
        assert_eq!(amount("15699.999999999998").unwrap(), U256::from(15700u64));
        assert_eq!(amount("1004999.9999999999").unwrap(), U256::from(1005000u64));
        assert!(amount("-1").is_err());
        assert!(amount("free").is_err());
        assert!(amount("12.x").is_err());
    }

    #[test]
    fn matching_pairs_on_scheme_and_network() {
        let payment = sign(&key(), &requirements(), 1).unwrap();
        let mut elsewhere = requirements();
        elsewhere.network = Network::Base;
        let mut other_scheme = requirements();
        other_scheme.scheme = "upto".to_string();

        assert!(matching(&[elsewhere.clone(), requirements()], &payment).is_some());
        assert!(matching(&[elsewhere, other_scheme], &payment).is_none());
        assert!(matching(&[], &payment).is_none());
    }
}
