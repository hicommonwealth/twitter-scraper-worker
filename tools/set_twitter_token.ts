import { WsProvider, ApiPromise } from '@polkadot/api';
import { TypeRegistry } from '@polkadot/types';

async function initApi() {
  const provider = new WsProvider('ws://localhost:9944');
  let unsubscribe: () => void;
  await new Promise((resolve) => {
    unsubscribe = provider.on('connected', () => resolve());
  });
  if (unsubscribe) unsubscribe();

  const registry = new TypeRegistry();
  const api = new ApiPromise({
    provider,
    registry,
    types: {
      Address: 'LookupSource',
      LookupSource: 'GenericLookupSource',
    }
  });
  return api.isReady;
}

async function main() {
  const api = await initApi();
  // specifically we need the "Bearer" token
  let token = process.env.TWITTER_TOKEN;
  await api.rpc.offchain.localStorageSet('PERSISTENT', 'identity-worker::twitter-oauth', token);}

main().then(() => {
  console.log('Main function done executing');
})