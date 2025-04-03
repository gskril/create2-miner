use alloy_primitives::{keccak256, Address, B256};
use anyhow::Result;

use crate::constants::{create3_factory, FACTORY_BYTECODE};

/// Compute CREATE2 address according to EIP-1014
pub fn compute_create2_address(deployer: Address, salt: B256, bytecode_hash: B256) -> Address {
    // let init_code_hash = keccak256(bytecode);

    // keccak256(0xff ++ deployer ++ salt ++ keccak256(bytecode))[12:]
    let mut input = Vec::with_capacity(1 + 20 + 32 + 32);
    input.push(0xff);
    input.extend_from_slice(deployer.as_slice());
    input.extend_from_slice(salt.as_slice());
    input.extend_from_slice(bytecode_hash.as_slice());

    let hash = keccak256(&input);
    Address::from_slice(&hash.as_slice()[12..])
}

/// Create a namespaced salt by hashing the original salt with a namespace string
pub fn create_arbitrary_namespaced_salt(salt: B256, namespace: &str) -> B256 {
    let mut input = Vec::with_capacity(32 + namespace.len());
    input.extend_from_slice(salt.as_slice());
    input.extend_from_slice(namespace.as_bytes());
    keccak256(&input)
}

/// Compute CREATE3 address
pub fn compute_create3_address(
    deployer: Address,
    salt: B256,
    namespace: Option<&str>,
) -> Result<Address> {
    // First hash the salt with the namespace if provided
    let arbitrary_namespaced_salt = if let Some(namespace) = namespace {
        if !namespace.is_empty() {
            create_arbitrary_namespaced_salt(salt, namespace)
        } else {
            salt
        }
    } else {
        salt
    };

    // Then hash with the deployer address
    let mut input = Vec::with_capacity(20 + 32);
    input.extend_from_slice(deployer.as_slice());
    input.extend_from_slice(arbitrary_namespaced_salt.as_slice());
    let namespaced_salt = keccak256(&input);

    // Compute proxy address using CREATE2
    let factory = create3_factory();
    let bytecode = keccak256(hex::decode(&FACTORY_BYTECODE[2..]).expect("Invalid bytecode hex"));
    let proxy_address = compute_create2_address(factory, namespaced_salt, bytecode);

    // Finally compute the contract address that will be deployed by the proxy
    // This follows the RLP encoding rules for contract addresses created by CREATE
    // prefix ++ address ++ nonce, where:
    // prefix = 0xd6 (0xc0 + 0x16), where 0x16 is length of: 0x94 ++ address ++ 0x01
    // 0x94 = 0x80 + 0x14 (0x14 is the length of an address)
    let mut rlp = Vec::with_capacity(22); // 1 + 1 + 20 + 1
    rlp.push(0xd6); // prefix
    rlp.push(0x94); // address marker
    rlp.extend_from_slice(proxy_address.as_slice());
    rlp.push(0x01); // nonce

    Ok(Address::from_slice(&keccak256(&rlp).as_slice()[12..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_NAMESPACE: &str = "L2ReverseRegistrar v1.0.0";
    const TEST_DEPLOYER: &str = "0x343431e9CEb7C19cC8d3eA0EE231bfF82B584910";
    const TEST_SALT: &str = "0x7cc6b9a2afa05a889a0394c767107d001d86bf77bea0141c11c296d3a8f72dac";

    // Helper function to get TEST_DEPLOYER as Address
    fn test_deployer() -> Address {
        TEST_DEPLOYER
            .parse()
            .expect("Invalid test deployer address")
    }

    #[test]
    fn test_create2_address_computation() {
        let deployer = create3_factory();
        let salt_bytes = keccak256(b"test_salt").to_vec();
        let salt = B256::from_slice(&salt_bytes);
        let bytecode =
            keccak256(hex::decode(&FACTORY_BYTECODE[2..]).expect("Invalid bytecode hex"));

        let address = compute_create2_address(deployer, salt, bytecode);

        // Address should be 20 bytes (40 hex chars without 0x prefix)
        assert_eq!(format!("{:x}", address).len(), 40);
        assert!(address.to_string().starts_with("0x"));
    }

    #[test]
    fn test_namespaced_salt_generation() {
        let salt_bytes = keccak256(b"test_salt").to_vec();
        let original_salt = B256::from_slice(&salt_bytes);
        let namespace = "test_namespace";

        let namespaced_salt = create_arbitrary_namespaced_salt(original_salt, namespace);
        let different_namespace_salt =
            create_arbitrary_namespaced_salt(original_salt, "different_namespace");

        // Same salt with different namespaces should produce different results
        assert_ne!(namespaced_salt, different_namespace_salt);

        // Same salt and namespace should produce same result
        let namespaced_salt_2 = create_arbitrary_namespaced_salt(original_salt, namespace);
        assert_eq!(namespaced_salt, namespaced_salt_2);
    }

    #[test]
    fn test_create3_address_computation() {
        let deployer = test_deployer();
        let salt = TEST_SALT.parse::<B256>().expect("Invalid test salt");

        // Test without namespace
        let no_namespace_address = compute_create3_address(deployer, salt, Some(""))
            .expect("Failed to compute non-namespaced address");
        assert_eq!(
            no_namespace_address.to_string().to_lowercase(),
            "0xAB6528783ac2a0BEf235ada3E1A5F6d8a623867E".to_lowercase(),
            "Non-namespaced address does not match TypeScript implementation"
        );

        // Test with namespace
        let namespaced_address = compute_create3_address(deployer, salt, Some(TEST_NAMESPACE))
            .expect("Failed to compute namespaced address");
        assert_eq!(
            namespaced_address.to_string().to_lowercase(),
            "0x5678e8193257f8b43eea3c5873b59ebcc18a0043".to_lowercase(),
            "Namespaced address does not match TypeScript implementation"
        );

        // Test None namespace (should behave same as empty string)
        let none_namespace_address = compute_create3_address(deployer, salt, None)
            .expect("Failed to compute address with None namespace");
        assert_eq!(
            none_namespace_address, no_namespace_address,
            "None namespace should produce same result as empty string namespace"
        );
    }

    #[test]
    fn test_create3_address_determinism() {
        let deployer = test_deployer();
        let salt = TEST_SALT.parse::<B256>().expect("Invalid test salt");

        // Same inputs should produce same outputs
        let address1 = compute_create3_address(deployer, salt, Some(TEST_NAMESPACE))
            .expect("Failed to compute first address");
        let address2 = compute_create3_address(deployer, salt, Some(TEST_NAMESPACE))
            .expect("Failed to compute second address");

        assert_eq!(
            address1, address2,
            "Same inputs should produce same address"
        );

        // Different namespace should produce different address
        let different_namespace_address =
            compute_create3_address(deployer, salt, Some("different_namespace"))
                .expect("Failed to compute address with different namespace");
        assert_ne!(
            address1, different_namespace_address,
            "Different namespace should produce different address"
        );
    }
}
