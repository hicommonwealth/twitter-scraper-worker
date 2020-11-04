#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::{
	self as system,
	offchain::{
		AppCrypto, CreateSignedTransaction,
	}
};
use frame_support::{
	debug, decl_module, decl_storage, decl_event,
	traits::Get,
};
use sp_runtime::{
	transaction_validity::{
		ValidTransaction, TransactionValidity, TransactionSource,
		TransactionPriority,
	},
	offchain::{
		storage::StorageValueRef,
		http, Duration
	}
};
use sp_std::vec::Vec;
use sp_io::crypto::sr25519_verify;
use sp_core::sr25519::{Public, Signature};
use sp_core::crypto::{KeyTypeId, UncheckedFrom};
use lite_json::JsonValue;

#[cfg(test)]
mod tests;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"twtr");
pub mod crypto {
        use super::KEY_TYPE;
        use sp_runtime::{
                app_crypto::{app_crypto, sr25519},
                traits::Verify,
        };
        use sp_core::sr25519::Signature as Sr25519Signature;
        app_crypto!(sr25519, KEY_TYPE);

        pub struct TestAuthId;
        impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature> for TestAuthId {
                type RuntimeAppPublic = Public;
                type GenericSignature = sp_core::sr25519::Signature;
                type GenericPublic = sp_core::sr25519::Public;
        }
}

/// This pallet's configuration trait
pub trait Trait: CreateSignedTransaction<Call<Self>> {
	/// The identifier type for an offchain worker.
	type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	/// The overarching dispatch call type.
	type Call: From<Call<Self>>;

	// Configuration parameters

	/// A grace period after we send transaction.
	///
	/// To avoid sending too many transactions, we only attempt to send one
	/// every `GRACE_PERIOD` blocks. We use Local Storage to coordinate
	/// sending between distinct runs of this offchain worker.
	type GracePeriod: Get<Self::BlockNumber>;

	/// Number of blocks of cooldown after unsigned transaction is included.
	///
	/// This ensures that we only accept unsigned transactions once, every `UnsignedInterval` blocks.
	type UnsignedInterval: Get<Self::BlockNumber>;

	/// A configuration for base priority of unsigned transactions.
	///
	/// This is exposed so that it can be tuned for particular runtime, when
	/// multiple pallets send unsigned transactions.
	type UnsignedPriority: Get<TransactionPriority>;
}

decl_storage! {
	trait Store for Module<T: Trait> as WorkerModule {}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId {
		NewHeader(u32, AccountId),
	}
);

static TWITTER_HANDLE: &'static str = "@HeyEdgeware";

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// // Errors must be initialized if they are used by the pallet.
		// type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;

		/// Offchain Worker entry point.
		///
		/// By implementing `fn offchain_worker` within `decl_module!` you declare a new offchain
		/// worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// succesfuly imported.
		/// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
		/// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
		/// so the code should be able to handle that.
		/// You can use `Local Storage` API to coordinate runs of the worker.
		fn offchain_worker(block_number: T::BlockNumber) {
			// It's a good idea to add logs to your offchain workers.
			// Using the `frame_support::debug` module you have access to the same API exposed by
			// the `log` crate.
			// Note that having logs compiled to WASM may cause the size of the blob to increase
			// significantly. You can use `RuntimeDebug` custom derive to hide details of the types
			// in WASM or use `debug::native` namespace to produce logs only when the worker is
			// running natively.
			debug::native::info!("Hello World from offchain workers!");

			// Since off-chain workers are just part of the runtime code, they have direct access
			// to the storage and other included pallets.
			//
			// We can easily import `frame_system` and retrieve a block hash of the parent block.
			let parent_hash = <system::Module<T>>::block_hash(block_number - 1.into());
			debug::debug!("Current block: {:?} (parent hash: {:?})", block_number, parent_hash);
		
			let result = Self::run(TWITTER_HANDLE.as_bytes().to_vec());
			if result == Ok(()) {
				debug::info!("Tweet fetching went OK.");
			} else {
				debug::warn!("Tweet fetching encountered error.");
			}
		}
	}
}

impl<T: Trait> Module<T> {
	fn run(mentions: Vec<u8>) -> Result<(), http::Error> {
		// obtain storage key for Twitter API call
		let s_info = StorageValueRef::persistent(b"identity-worker::twitter-oauth");
		let s_value = s_info.get::<Vec<u8>>();
		let mut authorization = Vec::new();
		if let Some(Some(twitter_key)) = s_value {
			// add "Bearer" prefix to key
			authorization.extend(b"Bearer ");
			authorization.extend(&twitter_key);
		} else {
			debug::info!("No Twitter OAuth key found.");
			return Err(http::Error::Unknown);
		}

		let authorization_str = sp_std::str::from_utf8(&authorization).map_err(|_| {
			debug::warn!("No UTF8 url");
			http::Error::Unknown
		})?;

		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
		// TODO: add since_id/until_id settings
		let mut url = Vec::new();
		let base_url = b"https://api.twitter.com/2/tweets/search/recent?limit=10&query=".to_vec();
		url.extend(base_url);
		url.extend(mentions);

		let url_str = sp_std::str::from_utf8(&url).map_err(|_| {
			debug::warn!("No UTF8 url");
			http::Error::Unknown
		})?;

		debug::native::info!("Querying URL: {:?}", url_str);
		let request = http::Request::get(url_str);
		let pending = request
			.add_header("Authorization", authorization_str)
			.deadline(deadline)
			.send().map_err(|_| http::Error::IoError)?;
		
		
		let response = pending.try_wait(deadline).map_err(|_| {
			debug::warn!("Request error / deadline reached");
			http::Error::DeadlineReached
		})??;
		if response.code != 200 {
			debug::warn!("Unexpected status code: {}", response.code);
			let body = response.body().collect::<Vec<u8>>();
			let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
				debug::warn!("No UTF8 body");
				http::Error::Unknown
			})?;
			debug::warn!("Error text: {:?}", body_str);
			return Err(http::Error::Unknown);
		}

		// Collect response body and parse/verify the signature text
		let body = response.body().collect::<Vec<u8>>();
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			debug::warn!("No UTF8 body");
			http::Error::Unknown
		})?;
		debug::info!("Got response body: {:?}", body_str);

		// interpret body string as a JSON blob
		// the base64 string should be found under "response_json.text" after the @handle
		let response_json = lite_json::parse_json(&body_str).unwrap();

		// get response.data: array of tweets
		let data = match response_json {
			JsonValue::Object(obj) => {
				obj.into_iter()
					.find(|(k, _)| k.iter().map(|c| *c as u8).collect::<Vec<u8>>() == b"data".to_vec())
					.and_then(|v| {
						match v.1 {
							JsonValue::Array(arr) => Some(arr),
							_ => None,
						}
					})
			},
			_ => None,
		}.unwrap_or(Vec::new());

		for tweet in data {
			let text = match tweet {
				JsonValue::Object(obj) => {
					obj.into_iter()
						.find(|(k, _)| k.iter().map(|c| *c as u8).collect::<Vec<u8>>() == b"text".to_vec())
						.and_then(|v| {
							match v.1 {
								JsonValue::String(t) => Some(t),
								_ => None,
							}
						})
				},
				_ => None,
			}.unwrap().into_iter().map(|c| c as u8).collect::<Vec<u8>>();

			// parse out base64 string: should be the second str if split on whitespace
			let text_str = sp_std::str::from_utf8(&text).map_err(|_| {
				debug::warn!("No UTF8 text");
				http::Error::Unknown
			})?;

			// TODO: add more complex parsing logic
			let signature = text_str.split_whitespace().nth(1).unwrap();
			debug::native::info!("Verifying signature: {:?}", signature);
			// interpret the entire string as a base64 encoded "signature | public key"
			// allocate a buffer of sufficient size -- signature is 64 bytes, pubkey is 32 bytes
			let mut buf: [u8; 96] = [0; 96];
			base64::decode_config_slice(signature, base64::STANDARD, &mut buf).unwrap();

			// verify signature matches
			let mut pub_bytes: [u8; 32] = [0; 32];
			pub_bytes.copy_from_slice(&buf[64..]);
			let public: Public = Public::unchecked_from(pub_bytes);
			let mut sig_bytes: [u8; 64] = [0; 64];
			sig_bytes.copy_from_slice(&buf[..64]);
			let signature = Signature::from_raw(sig_bytes);
			let is_valid = sr25519_verify(&signature, &public, &public);
			// TODO: handle results somehow?
			debug::native::info!("Signature for {:?} is valid? {:?}", public, is_valid);
		}
		Ok(())
	}
}

#[allow(deprecated)] // ValidateUnsigned
impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	/// Validate unsigned call to this module.
	///
	/// By default unsigned transactions are disallowed, but implementing the validator
	/// here we make sure that some particular calls (the ones produced by offchain worker)
	/// are being whitelisted and marked as valid.
	fn validate_unsigned(
		_source: TransactionSource,
		_call: &Self::Call,
	) -> TransactionValidity {
		ValidTransaction::with_tag_prefix("ExampleOffchainWorker")
		// We set base priority to 2**20 and hope it's included before any other
		// transactions in the pool. Next we tweak the priority depending on how much
		// it differs from the current average. (the more it differs the more priority it
		// has).
		.priority(T::UnsignedPriority::get())
		// The transaction is only valid for next 5 blocks. After that it's
		// going to be revalidated by the pool.
		.longevity(5)
		// It's fine to propagate that transaction to other peers, which means it can be
		// created even by nodes that don't produce blocks.
		// Note that sometimes it's better to keep it for yourself (if you are the block
		// producer), since for instance in some schemes others may copy your solution and
		// claim a reward.
		.propagate(true)
		.build()
	}
}