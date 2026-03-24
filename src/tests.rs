// tests.rs
// Unit tests for RustChatPro
// Run with: cargo test

#[cfg(test)]
mod tests {
    use crate::crypto::{decrypt, encrypt, RoomKey};
    use crate::types::{ChatMessage, MessageKind};

    // Test that encryption and decryption work correctly
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key       = RoomKey::generate();
        let plaintext = "Hello RustChatPro!";

        let encrypted = encrypt(plaintext, &key)
            .expect("encryption should succeed");

        // Encrypted text should not equal plaintext
        assert_ne!(encrypted, plaintext);

        let decrypted = decrypt(&encrypted, &key)
            .expect("decryption should succeed");

        // Decrypted text should equal original
        assert_eq!(decrypted, plaintext);
    }

    // Test that different messages produce different ciphertext
    // even with the same key (due to random nonce)
    #[test]
    fn test_different_nonces() {
        let key = RoomKey::generate();
        let msg = "same message";

        let enc1 = encrypt(msg, &key).unwrap();
        let enc2 = encrypt(msg, &key).unwrap();

        // Same plaintext should produce different ciphertext
        assert_ne!(enc1, enc2,
            "same message should encrypt differently each time");
    }

    // Test that wrong key cannot decrypt
    #[test]
    fn test_wrong_key_fails() {
        let key1 = RoomKey::generate();
        let key2 = RoomKey::generate();

        let encrypted = encrypt("secret", &key1).unwrap();
        let result    = decrypt(&encrypted, &key2);

        assert!(result.is_err(),
            "wrong key should fail to decrypt");
    }

    // Test RoomKey hex conversion
    #[test]
    fn test_room_key_hex_roundtrip() {
        let key     = RoomKey::generate();
        let hex     = key.to_hex();
        let key2    = RoomKey::from_hex(&hex)
            .expect("hex roundtrip should work");

        assert_eq!(key.bytes, key2.bytes,
            "key should survive hex roundtrip");
    }

    // Test ChatMessage constructors
    #[test]
    fn test_chat_message_chat() {
        let msg = ChatMessage::chat(
            "hello", "lobby", "abc123"
        );
        assert_eq!(msg.kind, MessageKind::Chat);
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.room, "lobby");
        assert_eq!(msg.sender_id, "abc123");
        assert!(msg.target_id.is_empty());
    }

    #[test]
    fn test_chat_message_dm() {
        let msg = ChatMessage::dm(
            "secret", "sender123", "target456"
        );
        assert_eq!(msg.kind, MessageKind::DirectMessage);
        assert_eq!(msg.content, "secret");
        assert_eq!(msg.sender_id, "sender123");
        assert_eq!(msg.target_id, "target456");
        assert!(msg.room.is_empty());
    }

    #[test]
    fn test_chat_message_system() {
        let msg = ChatMessage::system("server started");
        assert_eq!(msg.kind, MessageKind::System);
        assert_eq!(msg.sender_id, "server");
        assert_eq!(msg.content, "server started");
    }

    #[test]
    fn test_chat_message_ack() {
        let msg = ChatMessage::ack("received: hello");
        assert_eq!(msg.kind, MessageKind::Ack);
        assert_eq!(msg.content, "received: hello");
    }

    // Test display formatting
    #[test]
    fn test_display_chat_format() {
        let msg = ChatMessage::chat(
            "hello", "lobby", "abc12345-xxxx"
        );
        let display = msg.display();
        assert!(display.contains("[lobby]"),
            "display should contain room name");
        assert!(display.contains("hello"),
            "display should contain content");
        assert!(display.contains("abc12345"),
            "display should contain sender id");
    }

    #[test]
    fn test_display_system_format() {
        let msg     = ChatMessage::system("test event");
        let display = msg.display();
        assert!(display.contains("***"),
            "system messages should have *** markers");
        assert!(display.contains("test event"));
    }

    // Test empty message encryption
    #[test]
    fn test_encrypt_empty_string() {
        let key       = RoomKey::generate();
        let encrypted = encrypt("", &key)
            .expect("should encrypt empty string");
        let decrypted = decrypt(&encrypted, &key)
            .expect("should decrypt empty string");
        assert_eq!(decrypted, "");
    }

    // Test long message encryption
    #[test]
    fn test_encrypt_long_message() {
        let key  = RoomKey::generate();
        let long = "a".repeat(10000);
        let enc  = encrypt(&long, &key).unwrap();
        let dec  = decrypt(&enc, &key).unwrap();
        assert_eq!(dec, long);
    }
}