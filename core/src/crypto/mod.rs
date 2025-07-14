//! 加密与认证模块，提供端到端加密、密钥管理和安全认证功能

use crate::error::{Error, Result};
use log::error;
use sodiumoxide::crypto::{
    box_::{self, PublicKey, SecretKey},
    sealedbox,
    sign::{self, Signature},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 初始化加密模块
pub fn init() {
    if let Err(e) = sodiumoxide::init() {
        error!("初始化sodiumoxide失败: {:?}", e);
        panic!("初始化加密库失败");
    }
}

/// 密钥对
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// 公钥
    pub public_key: PublicKey,
    /// 私钥
    secret_key: SecretKey,
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

    /// 从Base64编码的字符串解析公钥
    pub fn public_key_from_base64(encoded: &str) -> Result<PublicKey> {
        let bytes = match base64::decode(encoded) {
            Ok(b) => b,
            Err(e) => {
                error!("Base64解码失败: {:?}", e);
                return Err(Error::Crypto("无效的Base64编码".to_string()));
            }
        };

        if bytes.len() != box_::PUBLICKEYBYTES {
            error!(
                "公钥长度不正确: {} != {}",
                bytes.len(),
                box_::PUBLICKEYBYTES
            );
            return Err(Error::Crypto("无效的公钥长度".to_string()));
        }

        let mut key_bytes = [0u8; box_::PUBLICKEYBYTES];
        key_bytes.copy_from_slice(&bytes);

        match PublicKey::from_slice(&key_bytes) {
            Some(key) => Ok(key),
            None => Err(Error::Crypto("无效的公钥格式".to_string())),
        }
    }
}

/// 签名密钥对
#[derive(Debug, Clone)]
pub struct SignKeyPair {
    /// 验证密钥
    pub public_key: sign::PublicKey,
    /// 签名密钥
    secret_key: sign::SecretKey,
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

    /// 签名数据
    pub fn sign(&self, data: &[u8]) -> Signature {
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

    /// 从Base64编码的字符串解析验证密钥
    pub fn public_key_from_base64(encoded: &str) -> Result<sign::PublicKey> {
        let bytes = match base64::decode(encoded) {
            Ok(b) => b,
            Err(e) => {
                error!("Base64解码失败: {:?}", e);
                return Err(Error::Crypto("无效的Base64编码".to_string()));
            }
        };

        if bytes.len() != sign::PUBLICKEYBYTES {
            error!(
                "验证密钥长度不正确: {} != {}",
                bytes.len(),
                sign::PUBLICKEYBYTES
            );
            return Err(Error::Crypto("无效的验证密钥长度".to_string()));
        }

        let mut key_bytes = [0u8; sign::PUBLICKEYBYTES];
        key_bytes.copy_from_slice(&bytes);

        match sign::PublicKey::from_slice(&key_bytes) {
            Some(key) => Ok(key),
            None => Err(Error::Crypto("无效的验证密钥格式".to_string())),
        }
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

    /// 获取公钥信息
    pub fn get_public_keys(&self) -> (String, String) {
        (
            self.encryption_keypair.public_key_base64(),
            self.signing_keypair.public_key_base64(),
        )
    }

    /// 计算与远程设备的共享密钥
    pub fn compute_shared_key(&self, device_id: &str, remote_public_key: &PublicKey) -> Result<()> {
        let _shared_key = box_::precompute(remote_public_key, &self.encryption_keypair.secret_key);

        let mut keys = match self.shared_keys.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取共享密钥锁失败: {:?}", e);
                return Err(Error::Crypto("获取共享密钥锁失败".to_string()));
            }
        };

        keys.insert(device_id.to_string(), (remote_public_key.clone(), self.encryption_keypair.secret_key.clone()));
        Ok(())
    }

    /// 加密数据
    pub fn encrypt(&self, device_id: &str, data: &[u8]) -> Result<Vec<u8>> {
        // 获取共享密钥
        let shared_key = {
            let keys = match self.shared_keys.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("获取共享密钥锁失败: {:?}", e);
                    return Err(Error::Crypto("获取共享密钥锁失败".to_string()));
                }
            };

            match keys.get(device_id) {
                Some(key) => key.clone(),
                None => {
                    return Err(Error::Crypto(format!(
                        "未找到设备的共享密钥: {}",
                        device_id
                    )))
                }
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
            return Err(Error::Crypto("加密数据长度不足".to_string()));
        }

        // 提取nonce
        let mut nonce = [0u8; box_::NONCEBYTES];
        nonce.copy_from_slice(&encrypted_data[..box_::NONCEBYTES]);
        let nonce = box_::Nonce::from_slice(&nonce).unwrap();

        // 提取加密数据
        let encrypted = &encrypted_data[box_::NONCEBYTES..];

        // 获取共享密钥
        let shared_key = {
            let keys = match self.shared_keys.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("获取共享密钥锁失败: {:?}", e);
                    return Err(Error::Crypto("获取共享密钥锁失败".to_string()));
                }
            };

            match keys.get(device_id) {
                Some(key) => key.clone(),
                None => {
                    return Err(Error::Crypto(format!(
                        "未找到设备的共享密钥: {}",
                        device_id
                    )))
                }
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
    pub fn verify(
        &self,
        signature: &[u8],
        data: &[u8],
        public_key_base64: &str,
    ) -> Result<bool> {
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
        let encrypted = sealedbox::seal(data, &public_key);
        Ok(encrypted)
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
