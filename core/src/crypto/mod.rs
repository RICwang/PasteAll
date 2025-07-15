//! 加密与认证模块，提供端到端加密、密钥管理和安全认证功能

use crate::error::{Error, Result};
use log::error;
use sodiumoxide::crypto::{
    box_::{self, PublicKey, SecretKey},
    sealedbox,
    sign::{self, Signature},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

// 全局密钥管理器单例
static CRYPTO_MANAGER: OnceLock<CryptoManager> = OnceLock::new();

/// 初始化加密模块
pub fn init() {
    if let Err(e) = sodiumoxide::init() {
        error!("初始化sodiumoxide失败: {e:?}");
        panic!("初始化加密库失败");
    }
    
    // 初始化全局密钥管理器
    let _ = CRYPTO_MANAGER.get_or_init(|| CryptoManager::new());
}

/// 获取全局密钥管理器引用
fn get_crypto_manager() -> &'static CryptoManager {
    CRYPTO_MANAGER.get().expect("加密模块未初始化")
}

/// 获取公钥（Base64编码）
pub fn get_public_key() -> Result<String> {
    Ok(get_crypto_manager().get_public_key_base64())
}

/// 签名数据
pub fn sign(data: &str, _public_key: &str) -> Result<String> {
    // 在实际应用中，需要验证public_key与当前设备匹配
    let signature = get_crypto_manager().sign(data.as_bytes());
    Ok(base64::encode(&signature))
}

/// 验证签名
pub fn verify_signature(data: &str, signature: &str, public_key: &str) -> Result<bool> {
    let signature_bytes = base64::decode(signature)
        .map_err(|e| Error::Crypto(format!("解析签名失败: {e}")))?;
    
    get_crypto_manager().verify(&signature_bytes, data.as_bytes(), public_key)
}

/// 加密数据
pub fn encrypt(device_id: &str, data: &[u8]) -> Result<Vec<u8>> {
    get_crypto_manager().encrypt(device_id, data)
}

/// 解密数据
pub fn decrypt(device_id: &str, encrypted_data: &[u8]) -> Result<Vec<u8>> {
    get_crypto_manager().decrypt(device_id, encrypted_data)
}

/// 为设备生成共享密钥
pub fn generate_shared_key(device_id: &str, remote_public_key_base64: &str) -> Result<()> {
    let remote_public_key = KeyPair::public_key_from_base64(remote_public_key_base64)?;
    get_crypto_manager().generate_shared_key(device_id, &remote_public_key)
}

/// 密钥对
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// 公钥
    pub public_key: PublicKey,
    /// 私钥
    pub secret_key: SecretKey,
}

impl KeyPair {
    /// 生成新的密钥对
    pub fn generate() -> Self {
        let (public_key, secret_key) = box_::gen_keypair();
        Self {
            public_key,
            secret_key,
        }
    }

    /// 获取公钥的Base64编码字符串
    pub fn public_key_base64(&self) -> String {
        base64::encode(self.public_key.as_ref())
    }

    /// 从Base64编码字符串解析公钥
    pub fn public_key_from_base64(base64_str: &str) -> Result<PublicKey> {
        let bytes = base64::decode(base64_str)
            .map_err(|e| Error::Crypto(format!("解析公钥Base64编码失败: {e}")))?;

        if bytes.len() != box_::PUBLICKEYBYTES {
            return Err(Error::Crypto(format!(
                "公钥长度不正确，期望 {} 字节，实际 {} 字节",
                box_::PUBLICKEYBYTES,
                bytes.len()
            )));
        }

        let mut pk_bytes = [0u8; box_::PUBLICKEYBYTES];
        pk_bytes.copy_from_slice(&bytes);

        Ok(PublicKey(pk_bytes))
    }
}

/// 签名密钥对
#[derive(Debug)]
pub struct SignKeyPair {
    /// 公钥
    pub public_key: sign::PublicKey,
    /// 私钥
    secret_key: sign::SecretKey,
}

impl Clone for SignKeyPair {
    fn clone(&self) -> Self {
        // 注意：这里重新创建了secret_key，实际上这不是真正的"克隆"
        // 在生产环境中，这可能需要更安全的实现方式
        Self {
            public_key: self.public_key,
            secret_key: self.secret_key.clone(),
        }
    }
}

impl SignKeyPair {
    /// 生成新的签名密钥对
    pub fn generate() -> Self {
        let (public_key, secret_key) = sign::gen_keypair();
        Self {
            public_key,
            secret_key,
        }
    }

    /// 使用私钥对消息进行签名
    pub fn sign(&self, data: &[u8]) -> sign::Signature {
        sign::sign_detached(data, &self.secret_key)
    }

    /// 验证签名
    pub fn verify(&self, signature: &Signature, data: &[u8]) -> bool {
        sign::verify_detached(signature, data, &self.public_key)
    }

    /// 获取公钥的Base64编码字符串
    pub fn public_key_base64(&self) -> String {
        base64::encode(self.public_key.as_ref())
    }

    /// 从Base64编码字符串解析公钥
    pub fn public_key_from_base64(base64_str: &str) -> Result<sign::PublicKey> {
        let bytes = base64::decode(base64_str)
            .map_err(|e| Error::Crypto(format!("解析公钥Base64编码失败: {e}")))?;

        if bytes.len() != sign::PUBLICKEYBYTES {
            return Err(Error::Crypto(format!(
                "公钥长度不正确，期望 {} 字节，实际 {} 字节",
                sign::PUBLICKEYBYTES,
                bytes.len()
            )));
        }

        let mut pk_bytes = [0u8; sign::PUBLICKEYBYTES];
        pk_bytes.copy_from_slice(&bytes);

        Ok(sign::PublicKey(pk_bytes))
    }
}

/// 加密管理器
pub struct CryptoManager {
    /// 加密密钥对
    encryption_keypair: KeyPair,
    /// 签名密钥对
    signing_keypair: SignKeyPair,
    /// 共享密钥缓存
    shared_keys: Arc<Mutex<HashMap<String, (PublicKey, SecretKey)>>>,
}

impl CryptoManager {
    /// 创建新的加密管理器
    pub fn new() -> Self {
        Self {
            encryption_keypair: KeyPair::generate(),
            signing_keypair: SignKeyPair::generate(),
            shared_keys: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 加载已有的密钥对
    pub fn with_keys(
        encryption_public: PublicKey,
        encryption_secret: SecretKey,
        signing_public: sign::PublicKey,
        signing_secret: sign::SecretKey,
    ) -> Self {
        Self {
            encryption_keypair: KeyPair {
                public_key: encryption_public,
                secret_key: encryption_secret,
            },
            signing_keypair: SignKeyPair {
                public_key: signing_public,
                secret_key: signing_secret,
            },
            shared_keys: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取加密公钥的Base64编码
    pub fn get_public_key_base64(&self) -> String {
        self.encryption_keypair.public_key_base64()
    }

    /// 获取签名公钥的Base64编码
    pub fn get_signing_key_base64(&self) -> String {
        self.signing_keypair.public_key_base64()
    }

    /// 获取公钥信息（向后兼容）
    pub fn get_public_keys(&self) -> (String, String) {
        (self.get_public_key_base64(), self.get_signing_key_base64())
    }

    /// 为设备生成共享密钥
    pub fn generate_shared_key(
        &self,
        device_id: &str,
        remote_public_key: &PublicKey,
    ) -> Result<()> {
        // 获取锁
        let mut keys = match self.shared_keys.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取共享密钥锁失败: {e:?}");
                return Err(Error::Crypto("获取共享密钥锁失败".to_string()));
            }
        };

        // 保存密钥
        keys.insert(
            device_id.to_string(),
            (
                *remote_public_key,
                self.encryption_keypair.secret_key.clone(),
            ),
        );
        Ok(())
    }

    /// 计算与远程设备的共享密钥（向后兼容）
    pub fn compute_shared_key(&self, device_id: &str, remote_public_key: &PublicKey) -> Result<()> {
        self.generate_shared_key(device_id, remote_public_key)
    }

    /// 加密数据
    pub fn encrypt(&self, device_id: &str, data: &[u8]) -> Result<Vec<u8>> {
        // 获取共享密钥
        let shared_key = {
            let keys = match self.shared_keys.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("获取共享密钥锁失败: {e:?}");
                    return Err(Error::Crypto("获取共享密钥锁失败".to_string()));
                }
            };

            match keys.get(device_id) {
                Some(key) => key.clone(),
                None => return Err(Error::Crypto(format!("未找到设备的共享密钥: {device_id}"))),
            }
        };

        // 生成随机nonce
        let nonce = box_::gen_nonce();

        // 使用共享密钥加密
        let precomputed_key = box_::precompute(&shared_key.0, &shared_key.1);
        let encrypted = box_::seal_precomputed(data, &nonce, &precomputed_key);

        // 组合nonce和加密数据
        let mut result = Vec::with_capacity(box_::NONCEBYTES + encrypted.len());
        result.extend_from_slice(nonce.as_ref());
        result.extend_from_slice(&encrypted);

        Ok(result)
    }

    /// 解密数据
    pub fn decrypt(&self, device_id: &str, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        // 检查数据长度
        if encrypted_data.len() < box_::NONCEBYTES {
            return Err(Error::Crypto("加密数据太短".to_string()));
        }

        // 提取nonce和加密数据
        let (nonce_bytes, encrypted) = encrypted_data.split_at(box_::NONCEBYTES);
        let nonce = match box_::Nonce::from_slice(nonce_bytes) {
            Some(n) => n,
            None => return Err(Error::Crypto("无效的nonce".to_string())),
        };

        // 获取共享密钥
        let shared_key = {
            let keys = match self.shared_keys.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("获取共享密钥锁失败: {e:?}");
                    return Err(Error::Crypto("获取共享密钥锁失败".to_string()));
                }
            };

            match keys.get(device_id) {
                Some(key) => key.clone(),
                None => return Err(Error::Crypto(format!("未找到设备的共享密钥: {device_id}"))),
            }
        };

        // 解密
        let precomputed_key = box_::precompute(&shared_key.0, &shared_key.1);
        match box_::open_precomputed(encrypted, &nonce, &precomputed_key) {
            Ok(decrypted) => Ok(decrypted),
            Err(_) => Err(Error::Crypto("解密失败".to_string())),
        }
    }

    /// 签名数据
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let signature = self.signing_keypair.sign(data);
        signature.as_ref().to_vec()
    }

    /// 验证签名
    pub fn verify(&self, signature: &[u8], data: &[u8], public_key_base64: &str) -> Result<bool> {
        // 解析验证密钥
        let public_key = SignKeyPair::public_key_from_base64(public_key_base64)?;

        // 创建签名对象
        let signature = match sign::Signature::from_bytes(signature) {
            Ok(sig) => sig,
            Err(_) => return Err(Error::Crypto("无效的签名格式".to_string())),
        };

        // 验证
        Ok(sign::verify_detached(&signature, data, &public_key))
    }

    /// 使用公钥加密数据（用于初始配对）
    pub fn encrypt_with_public_key(&self, public_key_base64: &str, data: &[u8]) -> Result<Vec<u8>> {
        // 解析公钥
        let public_key = KeyPair::public_key_from_base64(public_key_base64)?;

        // 加密
        Ok(sealedbox::seal(data, &public_key))
    }

    /// 使用私钥解密数据（用于初始配对）
    pub fn decrypt_with_private_key(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        match sealedbox::open(
            encrypted_data,
            &self.encryption_keypair.public_key,
            &self.encryption_keypair.secret_key,
        ) {
            Ok(decrypted) => Ok(decrypted),
            Err(_) => Err(Error::Crypto("解密失败".to_string())),
        }
    }

    /// 生成PIN码（用于配对验证）
    pub fn generate_pin() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let pin: u32 = rng.gen_range(100000..=999999);
        pin.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        let base64 = keypair.public_key_base64();

        assert!(!base64.is_empty());

        let result = KeyPair::public_key_from_base64(&base64);
        assert!(result.is_ok());
    }

    #[test]
    fn test_signing() {
        let keypair = SignKeyPair::generate();
        let data = b"test data";

        let signature = keypair.sign(data);
        let valid = keypair.verify(&signature, data);

        assert!(valid);
    }

    #[test]
    fn test_crypto_manager() {
        init();
        let manager = CryptoManager::new();
        let (enc_key, sign_key) = manager.get_public_keys();

        assert!(!enc_key.is_empty());
        assert!(!sign_key.is_empty());
    }
}
