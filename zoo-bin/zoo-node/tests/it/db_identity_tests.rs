use zoo_embedding::model_type::{EmbeddingModelType, OllamaTextEmbeddingsInference};
use zoo_message_primitives::schemas::identity::{StandardIdentity, StandardIdentityType};
use zoo_message_primitives::schemas::zoo_name::{ZooName, ZooSubidentityType};
use zoo_message_primitives::zoo_message::zoo_message_schemas::{IdentityPermissions, RegistrationCodeType};
use zoo_message_primitives::zoo_utils::encryption::{
    encryption_public_key_to_string, unsafe_deterministic_encryption_keypair,
};
use zoo_message_primitives::zoo_utils::signatures::{
    signature_public_key_to_string, unsafe_deterministic_signature_keypair,
};
use zoo_sqlite::errors::SqliteManagerError;
use zoo_sqlite::SqliteManager;

use std::path::PathBuf;
use std::sync::Arc;
use tempfile::NamedTempFile;

use ed25519_dalek::VerifyingKey;
use x25519_dalek::PublicKey as EncryptionPublicKey;

fn setup_test_db() -> SqliteManager {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = PathBuf::from(temp_file.path());
    let api_url = String::new();
    let model_type =
        EmbeddingModelType::OllamaTextEmbeddingsInference(OllamaTextEmbeddingsInference::SnowflakeArcticEmbedM);

    SqliteManager::new(db_path, api_url, model_type).unwrap()
}

async fn create_local_node_profile(
    db: &Arc<SqliteManager>,
    node_profile_name: String,
    encryption_public_key: EncryptionPublicKey,
    identity_public_key: VerifyingKey,
) {
    let node_profile = ZooName::new(node_profile_name.clone()).unwrap();
    match db.update_local_node_keys(node_profile, encryption_public_key, identity_public_key) {
        Ok(_) => (),
        Err(e) => panic!("Failed to update local node keys: {}", e),
    }
}

#[tokio::test]
async fn test_generate_and_use_registration_code_for_specific_profile() {
    let node_profile_name = "@@node1.zoo";
    let (_, identity_pk) = unsafe_deterministic_signature_keypair(0);
    let (_, encryption_pk) = unsafe_deterministic_encryption_keypair(0);
    let db = setup_test_db();
    let zoo_db = Arc::new(db);

    // Create a local node profile
    create_local_node_profile(&zoo_db, node_profile_name.to_string(), encryption_pk, identity_pk).await;

    let profile_name = "profile_1";
    let (_, profile_identity_pk) = unsafe_deterministic_signature_keypair(1);
    let (_, profile_encryption_pk) = unsafe_deterministic_encryption_keypair(1);

    // Test generate_registration_code_for_specific_profile
    let registration_code = zoo_db
        .generate_registration_new_code(IdentityPermissions::Admin, RegistrationCodeType::Profile)
        .unwrap();

    // Test use_registration_code
    zoo_db
        .use_registration_code(
            &registration_code,
            node_profile_name,
            profile_name,
            &signature_public_key_to_string(profile_identity_pk),
            &encryption_public_key_to_string(profile_encryption_pk),
            None,
            None,
        )
        .unwrap();

    // check in db
    let profile_name =
        ZooName::from_node_and_profile_names(node_profile_name.to_string(), profile_name.to_string()).unwrap();
    let permission_in_db: IdentityPermissions = zoo_db.get_profile_permission(profile_name).unwrap();
    assert_eq!(permission_in_db, IdentityPermissions::Admin);
}

#[tokio::test]
async fn test_generate_and_use_registration_code_for_device() {
    let node_profile_name = "@@node1.zoo";
    let (_identity_sk, identity_pk) = unsafe_deterministic_signature_keypair(0);
    let (_encryption_sk, encryption_pk) = unsafe_deterministic_encryption_keypair(0);
    let db = setup_test_db();
    let zoo_db = Arc::new(db);

    let profile_name = "profile_1";
    let (_profile_identity_sk, profile_identity_pk) = unsafe_deterministic_signature_keypair(1);
    let (_profile_encryption_sk, profile_encryption_pk) = unsafe_deterministic_encryption_keypair(1);

    let device_name = "device_1";
    let (_device_identity_sk, device_identity_pk) = unsafe_deterministic_signature_keypair(2);
    let (_device_encryption_sk, device_encryption_pk) = unsafe_deterministic_encryption_keypair(2);

    // first create the keys for the node
    create_local_node_profile(&zoo_db, node_profile_name.to_string(), encryption_pk, identity_pk).await;

    // second create a new profile (required to register a device)
    let registration_code = zoo_db
        .generate_registration_new_code(IdentityPermissions::Admin, RegistrationCodeType::Profile)
        .unwrap();

    let _profile_result = zoo_db.use_registration_code(
        &registration_code,
        node_profile_name,
        profile_name,
        &signature_public_key_to_string(profile_identity_pk),
        &encryption_public_key_to_string(profile_encryption_pk),
        Some(&signature_public_key_to_string(device_identity_pk)),
        Some(&encryption_public_key_to_string(device_encryption_pk)),
    );

    // registration code for device
    let registration_code = zoo_db
        .generate_registration_new_code(
            IdentityPermissions::Standard,
            RegistrationCodeType::Device(profile_name.to_string()),
        )
        .unwrap();

    // Test use_registration_code
    zoo_db
        .use_registration_code(
            &registration_code,
            node_profile_name,
            device_name,
            &signature_public_key_to_string(profile_identity_pk),
            &encryption_public_key_to_string(profile_encryption_pk),
            Some(&signature_public_key_to_string(device_identity_pk)),
            Some(&encryption_public_key_to_string(device_encryption_pk)),
        )
        .unwrap();

    // check in db
    let profile_name =
        ZooName::from_node_and_profile_names(node_profile_name.to_string(), profile_name.to_string()).unwrap();
    let permission_in_db = zoo_db.get_profile_permission(profile_name.clone()).unwrap();
    assert_eq!(permission_in_db, IdentityPermissions::Admin);

    // check device permission
    let device_name = ZooName::from_node_and_profile_names_and_type_and_name(
        node_profile_name.to_string(),
        profile_name.get_profile_name_string().unwrap().to_string(),
        ZooSubidentityType::Device,
        device_name.to_string(),
    )
    .unwrap();
    let permission_in_db = zoo_db.get_device_permission(device_name).unwrap();
    assert_eq!(permission_in_db, IdentityPermissions::Standard);
}

#[tokio::test]
async fn test_generate_and_use_registration_code_for_device_with_main_profile() {
    let node_profile_name = "@@node1.zoo";
    let (_identity_sk, identity_pk) = unsafe_deterministic_signature_keypair(0);
    let (_encryption_sk, encryption_pk) = unsafe_deterministic_encryption_keypair(0);
    let db = setup_test_db();
    let zoo_db = Arc::new(db);

    let (_profile_identity_sk, profile_identity_pk) = unsafe_deterministic_signature_keypair(1);
    let (_profile_encryption_sk, profile_encryption_pk) = unsafe_deterministic_encryption_keypair(1);

    let device_name = "device_main";
    let (_device_identity_sk, device_identity_pk) = unsafe_deterministic_signature_keypair(2);
    let (_device_encryption_sk, device_encryption_pk) = unsafe_deterministic_encryption_keypair(2);

    // Create the keys for the node
    create_local_node_profile(&zoo_db, node_profile_name.to_string(), encryption_pk, identity_pk).await;

    // Generate a registration code for device with main as profile_name
    let registration_code = zoo_db
        .generate_registration_new_code(
            IdentityPermissions::Standard,
            RegistrationCodeType::Device("main".to_string()),
        )
        .unwrap();

    // Use the registration code to create the device and automatically the "main" profile if not exists
    let _device_result = zoo_db.use_registration_code(
        &registration_code,
        node_profile_name,
        device_name,
        &signature_public_key_to_string(profile_identity_pk),
        &encryption_public_key_to_string(profile_encryption_pk),
        Some(&signature_public_key_to_string(device_identity_pk)),
        Some(&encryption_public_key_to_string(device_encryption_pk)),
    );

    // Check if "main" profile exists in db
    let main_profile_name =
        ZooName::from_node_and_profile_names(node_profile_name.to_string(), "main".to_string()).unwrap();
    let main_permission_in_db = zoo_db.get_profile_permission(main_profile_name.clone()).unwrap();
    assert_eq!(main_permission_in_db, IdentityPermissions::Admin);

    // Check if device exists in db
    let device_full_name = ZooName::from_node_and_profile_names_and_type_and_name(
        node_profile_name.to_string(),
        "main".to_string(),
        ZooSubidentityType::Device,
        device_name.to_string(),
    )
    .unwrap();
    let device_permission_in_db = zoo_db.get_device_permission(device_full_name.clone()).unwrap();
    assert_eq!(device_permission_in_db, IdentityPermissions::Standard);

    // Get the device from the database and check that it matches the expected values
    let device_in_db = zoo_db.get_device(device_full_name.clone()).unwrap();
    assert_eq!(device_in_db.device_signature_public_key, device_identity_pk);
    assert_eq!(device_in_db.device_encryption_public_key, device_encryption_pk);
    assert_eq!(device_in_db.permission_type, IdentityPermissions::Standard);

    assert_ne!(
        device_in_db.profile_encryption_public_key,
        device_in_db.device_encryption_public_key
    );
    assert_ne!(
        device_in_db.profile_signature_public_key,
        device_in_db.device_signature_public_key
    );
}

#[tokio::test]
async fn test_generate_and_use_registration_code_no_associated_profile() {
    let node_profile_name = "@@node1.zoo";
    let (_identity_sk, identity_pk) = unsafe_deterministic_signature_keypair(0);
    let (_encryption_sk, encryption_pk) = unsafe_deterministic_encryption_keypair(0);
    let db = setup_test_db();
    let zoo_db = Arc::new(db);

    let profile_name = "profile_1";
    let (_profile_identity_sk, _profile_identity_pk) = unsafe_deterministic_signature_keypair(1);
    let (_profile_encryption_sk, _profile_encryption_pk) = unsafe_deterministic_encryption_keypair(1);

    let device_name = "device_1";
    let (_device_identity_sk, device_identity_pk) = unsafe_deterministic_signature_keypair(2);
    let (_device_encryption_sk, device_encryption_pk) = unsafe_deterministic_encryption_keypair(2);

    // first create the keys for the node
    create_local_node_profile(&zoo_db, node_profile_name.to_string(), encryption_pk, identity_pk).await;

    // registration code for device
    let registration_code = zoo_db
        .generate_registration_new_code(
            IdentityPermissions::Standard,
            RegistrationCodeType::Device(profile_name.to_string()),
        )
        .unwrap();

    // Test use_registration_code
    let result = zoo_db.use_registration_code(
        &registration_code,
        node_profile_name,
        device_name,
        &signature_public_key_to_string(device_identity_pk),
        &encryption_public_key_to_string(device_encryption_pk),
        None,
        None,
    );

    // Check if an error is returned as no profile is associated
    assert!(
        matches!(result, Err(SqliteManagerError::ProfileNotFound(_node_profile_name))),
        "Expected ProfileNotFound error"
    );
}

#[tokio::test]
async fn test_new_load_all_sub_identities() {
    let node_profile_name = ZooName::new("@@node1.zoo".to_string()).unwrap();
    let (_identity_sk, identity_pk) = unsafe_deterministic_signature_keypair(0);
    let (_encryption_sk, encryption_pk) = unsafe_deterministic_encryption_keypair(0);
    let db = setup_test_db();
    let zoo_db = Arc::new(db);

    // Create a local node profile
    create_local_node_profile(
        &zoo_db,
        node_profile_name.clone().to_string(),
        encryption_pk,
        identity_pk,
    )
    .await;

    // Insert some sub-identities
    for i in 1..=5 {
        let (_subidentity_sk, subidentity_pk) = unsafe_deterministic_signature_keypair(i);
        let (_subencryption_sk, subencryption_pk) = unsafe_deterministic_encryption_keypair(i);
        let subidentity_name = format!("subidentity_{}", i);

        let identity = StandardIdentity::new(
            ZooName::from_node_and_profile_names(node_profile_name.to_string(), subidentity_name.to_string())
                .unwrap(),
            None,
            encryption_pk,
            identity_pk,
            Some(subencryption_pk),
            Some(subidentity_pk),
            StandardIdentityType::Profile,
            IdentityPermissions::Standard,
        );

        // check if there was an error and print to console
        zoo_db
            .insert_profile(identity)
            .unwrap_or_else(|e| println!("Error inserting sub-identity: {}", e));
    }

    // Test new_load_all_sub_identities
    let identities = zoo_db
        .get_all_profiles(node_profile_name.clone())
        .unwrap_or_else(|e| panic!("Error loading all sub-identities: {}", e));

    assert_eq!(identities.len(), 5);

    // add asserts to check if the identities are correct
    for i in 1..=5 {
        let (_subidentity_sk, subidentity_pk) = unsafe_deterministic_signature_keypair(i);
        let (_subencryption_sk, subencryption_pk) = unsafe_deterministic_encryption_keypair(i);
        let subidentity_name = format!("subidentity_{}", i);

        let identity = StandardIdentity::new(
            ZooName::from_node_and_profile_names(node_profile_name.to_string(), subidentity_name.to_string())
                .unwrap(),
            None,
            encryption_pk,
            identity_pk,
            Some(subencryption_pk),
            Some(subidentity_pk),
            // todo: review this
            StandardIdentityType::Profile,
            IdentityPermissions::Standard,
        );

        assert_eq!(identities[(i - 1) as usize], identity);
    }
}

#[tokio::test]
async fn test_new_insert_profile() {
    let node_profile_name = "@@node1.zoo";
    let (_identity_sk, identity_pk) = unsafe_deterministic_signature_keypair(0);
    let (_encryption_sk, encryption_pk) = unsafe_deterministic_encryption_keypair(0);
    let db = setup_test_db();
    let zoo_db = Arc::new(db);

    // Check if any profile exists before insertion
    assert!(
        !zoo_db.has_any_profile().unwrap(),
        "No profiles should exist before insertion"
    );

    let subidentity_name = "subidentity_1";
    let (_subidentity_sk, subidentity_pk) = unsafe_deterministic_signature_keypair(1);
    let (_subencryption_sk, subencryption_pk) = unsafe_deterministic_encryption_keypair(1);

    let identity = StandardIdentity::new(
        ZooName::from_node_and_profile_names(node_profile_name.to_string(), subidentity_name.to_string()).unwrap(),
        None,
        encryption_pk,
        identity_pk,
        Some(subencryption_pk),
        Some(subidentity_pk),
        StandardIdentityType::Profile,
        IdentityPermissions::Standard,
    );

    // Test insert_profile
    zoo_db.insert_profile(identity.clone()).unwrap();

    // Check if any profile exists after insertion
    assert!(
        zoo_db.has_any_profile().unwrap(),
        "A profile should exist after insertion"
    );
}

#[tokio::test]
async fn test_remove_profile() {
    let node_profile_name = "@@node1.zoo";
    let (_identity_sk, identity_pk) = unsafe_deterministic_signature_keypair(0);
    let (_encryption_sk, encryption_pk) = unsafe_deterministic_encryption_keypair(0);
    let db = setup_test_db();
    let zoo_db = Arc::new(db);

    let subidentity_name = "subidentity_1";
    let (_subidentity_sk, subidentity_pk) = unsafe_deterministic_signature_keypair(1);
    let (_subencryption_sk, subencryption_pk) = unsafe_deterministic_encryption_keypair(1);

    let identity = StandardIdentity::new(
        ZooName::from_node_and_profile_names(node_profile_name.to_string(), subidentity_name.to_string()).unwrap(),
        None,
        encryption_pk,
        identity_pk,
        Some(subencryption_pk),
        Some(subidentity_pk),
        StandardIdentityType::Profile,
        IdentityPermissions::Standard,
    );

    // Insert identity
    zoo_db.insert_profile(identity.clone()).unwrap();

    // Remove identity
    zoo_db.remove_profile(subidentity_name).unwrap();
}
