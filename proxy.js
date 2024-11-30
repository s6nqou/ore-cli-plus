const http = require('http');
const https = require('https');
const rpcPool = require('./rpc_list.json').default_rpc_list.map(url => new URL(url));

http.createServer((req, res) => {
    let chunks = [];
    req.on('data', chunk => {
        chunks.push(chunk);
    });
    req.on('end', () => {
        const url = new URL(req.url, `http://${req.headers.host}`);
        const id = Number(url.pathname.split('/')[1]);
        handleRpcRequest(chunks.join(''), id).then(data => {
            res.writeHead(200, {
                'content-type': 'application/json'
            });
            res.write(data);
            res.end();
        }).catch(err => {
            res.writeHead(500);
            res.write(JSON.stringify(err));
            res.end();
        })
    })
}).listen(8000);


class CachePool {
    constructor(mapKeyAge) {
        this.mapKeyAge = mapKeyAge;
        this.mapKeyCache = {};
    }
    get(key) {
        const cache = this.mapKeyCache[key];
        if (cache) {
            if (Date.now() > cache.timestamp + this.mapKeyAge[key]) {
                this.mapKeyCache[key] = null;
                return null;
            } else {
                return cache.value;
            }
        } else {
            return null;
        }
    }
    add(key, value) {
        if (typeof this.mapKeyAge[key] == 'number' && value) {
            this.mapKeyCache[key] = {
                timestamp: Date.now(),
                value
            }
        }
    }
}

const cacheConfig = {
    'getVersion': 120000,
    'getRecentPrioritizationFees': 5000,
    'getLatestBlockhash': 5000,
}

const cachePool = new CachePool(cacheConfig);

const handleRpcRequest = async (data, id) => {
    const method = JSON.parse(data).method;
    const cachable = cacheConfig[method] !== undefined;
    if (cachable) {
        const cachedValue = cachePool.get(method);
        if (cachedValue) {
            return cachedValue;
        }
    }
    const res = await requestFromRpcPool(data, id).catch(console.log);
    if (res && cachable) {
        cachePool.add(method, res);
    }
    return res;
}

const requestFromRpcPool = (data, id) => new Promise((resolve, reject) => {
    if (id === undefined || id == null) {
        id = Math.floor(Math.random() * rpcPool.length);
    }
    const rpc = rpcPool[id % rpcPool.length];
    const req = https.request({
        hostname: rpc.hostname,
        path: rpc.pathname,
        method: 'POST'
    }, res => {
        let chunks = [];
        res.on('data', chunk => {
            chunks.push(chunk);
        });
        res.on('end', () => {
            if (res.statusCode >= 200 && res.statusCode < 300) {
                resolve(chunks.join(''))
            } else {
                reject(chunks.join(''))
            }
        });
        res.on('error', reject);
    });
    req.on('error', reject);
    req.write(data);
    req.end();
});
