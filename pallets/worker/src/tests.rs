use crate::*;
use codec::{Encode, Decode};
use frame_support::{
	impl_outer_origin, parameter_types,
	weights::Weight,
};
use sp_core::{
	H256,
	offchain::{OffchainExt, testing},
	sr25519::{Signature},
};

use sp_runtime::{
	Perbill,
	testing::{Header, TestXt},
	offchain::http,
	traits::{
		BlakeTwo256, IdentityLookup, Extrinsic as ExtrinsicT,
		IdentifyAccount, Verify,
	},
};

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
}

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq, Encode, Decode)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
	type BaseCallFilter = ();
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = sp_core::sr25519::Public;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

type Extrinsic = TestXt<Call<Test>, ()>;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test where
	Call<Test>: From<LocalCall>,
{
	type OverarchingCall = Call<Test>;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test where
	Call<Test>: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call<Test>,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(Call<Test>, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}

parameter_types! {
	pub const GracePeriod: u64 = 5;
	pub const UnsignedInterval: u64 = 128;
	pub const UnsignedPriority: u64 = 1 << 20;
}

impl Trait for Test {
	type Event = ();
	type AuthorityId = crypto::TestAuthId;
	type Call = Call<Test>;
	type GracePeriod = GracePeriod;
	type UnsignedInterval = UnsignedInterval;
	type UnsignedPriority = UnsignedPriority;
}

type Example = Module<Test>;

#[test]
fn should_not_make_twitter_call_without_api_key() {
	let (offchain, _state) = testing::TestOffchainExt::new();
	let mut t = sp_io::TestExternalities::default();
	t.register_extension(OffchainExt::new(offchain));

	t.execute_with(|| {
		let mut s_info = StorageValueRef::persistent(b"identity-worker::twitter-oauth");
		s_info.clear();
	
		// when
    let result = Example::run();
		// then
		// TODO: make specific error for API key
		assert_eq!(result, Err(http::Error::Unknown));
	});
}

#[test]
fn should_make_twitter_call_and_parse_result() {
	let (offchain, state) = testing::TestOffchainExt::new();
	let mut t = sp_io::TestExternalities::default();
	t.register_extension(OffchainExt::new(offchain));

	set_twitter_response(&mut state.write());

	t.execute_with(|| {
		// set twitter API key in storage
		let api_key: Vec<u8> = b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_vec();
		let s_info = StorageValueRef::persistent(b"identity-worker::twitter-oauth");
		s_info.set(&api_key);
	
    // when
    
    // TODO: assert that the verified signature was for alice
		// let alice_pubkey = sp_core::sr25519::Pair::from_seed(b"12345678901234567890123456789012").public();
		// let bob_pubkey = sp_core::sr25519::Pair::from_seed(b"12345678901234567890123456789013").public();
    
    let result = Example::run();
		// then
		assert_eq!(result, Ok(()));
	});
}

fn set_twitter_response(state: &mut testing::OffchainState) {
	let data = br#"{
		"data": [
      {
			  "id": "1050118621198921728",
			  "text": "@testtestajlashdghwetiwjeijwtest 3o4mfx9gZVjp4QDToUhQr5elsGr0M4wKTySjI9kfOx3KNqdxnRYTHiZEQ2vbEoX6e+K+UKeomI4hjbshQWt6gHQcCKBvQcWWYI9ndCWb2QQzBK36XT7qYnYL2b6XY01j"
      }
    ]
	}"#.to_vec();
	let mut headers = Vec::new();
	headers.push((String::from("Authorization"), String::from("Bearer AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")));
	state.expect_request(testing::PendingRequest {
		method: "GET".into(),
		uri: "https://api.twitter.com/2/tweets/search/recent?query=@heyedgeware&limit=10".into(),
		response: Some(data),
		sent: true,
		headers: headers,
		..Default::default()
	});
}
