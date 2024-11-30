const fs = require('fs');
const path = require('path');
const child_process = require('child_process');

const keypairRoot = child_process.execSync(`source ${path.join(__dirname, '.env')}; echo $KEYPAIR_ROOT`, { encoding: 'utf8', shell: '/bin/bash' }).trim();
const keypairs = fs.readdirSync(keypairRoot);

const { default_rpc_list: defaultRPCList, submit_rpc_list: submitRPCList } = require(path.join(__dirname, 'rpc_list.json'));

module.exports = {
  apps: keypairs.map((keypair, i) => {
    const id = keypair.split('.')[0];
    const keypairPath = path.join(keypairRoot, keypair);
    const defaultRPC = defaultRPCList[i % defaultRPCList.length];
    const submitRPC = submitRPCList[i % submitRPCList.length];
    return {
      name: 'claim-' + id,
      script: './claim.sh',
      args: `${keypairPath} ${defaultRPC} ${submitRPC}`,
      cwd: __dirname,
      interpreter: '/bin/bash',
      autorestart: false,
    }
  })
};
