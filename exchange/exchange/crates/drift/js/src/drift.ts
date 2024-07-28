import {Connection, ConnectionConfig, FetchFn, Keypair, PublicKey, TransactionSignature} from '@solana/web3.js';
import {
    BN,
    configs,
    DriftClient,
    MarketType,
    PerpPosition,
    RetryTxSender,
    SpotPosition,
    UserAccount,
    Wallet,
    BulkAccountLoader,
    Order,
    OrderParams
} from "@drift-labs/sdk";
import bs58 from "bs58";
import * as process from "process";
import {AnchorProvider} from "@coral-xyz/anchor";
import {exec} from 'child_process';


function ensure_env(name: string): string {
    const val = process.env[name];
    // if val is undefined, null or empty string
    if (!val) {
        throw new Error(`Missing environment variable ${name}`);
    }
    return val;
}

const SOLANA_ENDPOINT_HELIUS = "https://cold-hanni-fast-mainnet.helius-rpc.com/";
const SOLANA_ENDPOINT_TRITON = "https://drift-drift-951a.mainnet.rpcpool.com/";

async function executeCommand(command: string) {
    console.debug("Executing command:", command)
    return await new Promise<string>((resolve, reject) => {
        exec(command, (error, stdout, stderr) => {
            if (error) {
                reject(stderr);
                return;
            }

            resolve(stdout);
        });
    });
}

// @ts-ignore
const makeRequest: FetchFn = async (url, options): Promise<Response> => {
    const output = await executeCommand(`curl -i '${url}'` +
        '  -H \'accept: */*\' ' +
        '  -H \'accept-language: en-US,en;q=0.9,zh-CN;q=0.8,zh;q=0.7\' ' +
        '  -H \'cache-control: no-cache\' ' +
        '  -H \'content-type: application/json\' ' +
        '  -H \'dnt: 1\' ' +
        '  -H \'origin: https://app.drift.trade\' ' +
        '  -H \'pragma: no-cache\' ' +
        '  -H \'referer: https://app.drift.trade/\' ' +
        '  -H \'sec-ch-ua: "Google Chrome";v="123", "Not:A-Brand";v="8", "Chromium";v="123"\' ' +
        '  -H \'sec-ch-ua-mobile: ?0\' ' +
        '  -H \'sec-ch-ua-platform: "macOS"\' ' +
        '  -H \'sec-fetch-dest: empty\' ' +
        '  -H \'sec-fetch-mode: cors\' ' +
        '  -H \'sec-fetch-site: cross-site\' ' +
        '  -H \'solana-client: js/0.0.0-development\' ' +
        '  -H \'user-agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36\'' +
        `  --data-raw '${options!.body}'`);
    // example output:
    // HTTP/2 200
    // content-type: application/json; charset=utf-8
    // vary: origin
    // content-length: 81
    // date: Thu, 11 Apr 2024 16:55:47 GMT
    // x-rpc-node: lb-sg5
    // access-control-allow-origin: https://app.drift.trade
    // access-control-allow-methods: OPTIONS, POST, GET
    // access-control-allow-headers: authorization, *
    // access-control-max-age: 86400
    // allow: OPTIONS, POST, GET
    //
    // {"jsonrpc":"2.0","result":259569599,"id":"a20073c9-d1bd-49b3-8cad-46e43af1b599"}

    console.debug("Response:", output)
    // split the output into protocol line, headers and body
    const [protocol_line, ...lines] = output.split('\n');
    const [protocol, status, statusText] = protocol_line.split(' ', 3);
    const headers = new Headers();
    let bodyLines = []
    let expectBody = false;
    for (const line of lines) {
        if (expectBody) {
            bodyLines.push(line)
            continue
        }
        if (line.trim() === '') {
            expectBody = true;
            continue
        }
        // console.debug("Header:", line)
        const [key, value] = line.split(': ', 2);
        headers.append(key, value);
    }
    const body = bodyLines.join('\n')
    // console.debug("Body:", body.length)
    const response = {
        url: url as string,
        redirected: false,
        type: 'basic',
        headers,
        status: Number(status),
        statusText: statusText.trim() || '',
        ok: status.startsWith('2'),
        text: async () => body,
        json: async () => JSON.parse(body)
    }
    // console.debug("Response:", response)
    // @ts-ignore
    return response
}

function getConnectionConfig(url: string): ConnectionConfig {
    if (url == SOLANA_ENDPOINT_TRITON) {
        return {
            commitment: "processed",
            fetch: makeRequest
        }
    } else {
        return {
            commitment: "processed",
        }
    }

}

export class Drift {
    client: DriftClient = null!;

    async init() {
        const DRIFT = process.argv[3] || "DRIFT";
        console.info("Using ENV Prefix:", DRIFT)
        const DRIFT_ADDRESS = ensure_env(`${DRIFT}_ADDRESS`);
        const authority = new PublicKey(DRIFT_ADDRESS);
        console.info('Drift Authority Address:', DRIFT_ADDRESS);

        let SOLANA_ENDPOINT = process.env['SOLANA_RPC_URL'] || null

        if (!SOLANA_ENDPOINT) {
            // get account balance and measure latency. Use the one with the lowest latency
            let latency = Number.MAX_VALUE;
            for (const endpoint of [SOLANA_ENDPOINT_HELIUS, SOLANA_ENDPOINT_TRITON]) {
                console.info(`Checking Solana Endpoint: ${endpoint}`);
                const start = Date.now();
                try {
                    const connection = new Connection(endpoint, getConnectionConfig(endpoint));
                    const end = Date.now();
                    await connection.getSlot()
                    const newLatency = end - start;
                    console.info(`Latency to ${endpoint}: ${newLatency}ms`);
                    if (newLatency < latency) {
                        latency = newLatency;
                        SOLANA_ENDPOINT = endpoint;
                    }
                } catch (e) {
                    console.error(`Failed to connect to ${endpoint}`, e);
                }
            }
        }
        console.info('Using Solana Endpoint:', SOLANA_ENDPOINT);

        const connection = new Connection(SOLANA_ENDPOINT!, getConnectionConfig(SOLANA_ENDPOINT!))
        const balance = await connection.getBalance(authority);
        console.info(`Balance of Authority: ${balance}`);
        if (balance == 0) {
            throw new Error(`Account ${DRIFT_ADDRESS} has no balance, double check the address and the network.`);
        }


        const DRIFT_PRIVATE_KEY = ensure_env(`${DRIFT}_PRIVATE_KEY`);
        const wallet = new Wallet(Keypair.fromSecretKey(bs58.decode(DRIFT_PRIVATE_KEY)));
        console.info('Fee Payer Address:', wallet.publicKey.toString());
        const balancePayer = await connection.getBalance(wallet.publicKey);
        console.info(`Balance of Fee Payer: ${balancePayer}`);
        if (balancePayer == 0) {
            throw new Error(`Fee Payer ${wallet.publicKey.toString()} has no balance, double check the address and the network.`);
        }

        const env = "mainnet-beta";
        const driftConfig = configs[env];
        const subaccount = Number(process.env[`${DRIFT}_SUBACCOUNT`] || '0');
        // priority price of 5000 macro Lamports should be enough
        const computeUnitsPrice = Number(process.env['SOLANA_COMPUTE_UNIT_PRICE'] || '5000')

        const programID = new PublicKey(driftConfig.DRIFT_PROGRAM_ID);
        this.client = new DriftClient({
            connection,
            wallet,
            programID,
            userStats: true,
            env,
            authority,
            activeSubAccountId: subaccount,
            includeDelegates: true,
            txSender: new RetryTxSender({
                connection,
                wallet,
                retrySleep: 500,
                timeout: 10000
            }),
            opts: {
                skipPreflight: true,
                commitment: 'processed',
                ...AnchorProvider.defaultOptions()
            },
            txParams: {
                computeUnitsPrice
            }
        });
        console.log("Initializing Drift Client");
        await this.client.addUser(subaccount);
        console.log("UserAccount Address:", (await this.client.getUserAccountPublicKey()).toString());
        await this.client.subscribe();
        console.log("Drift Client Subscribed");
    }

    async get_orders(): Promise<Order[]> {
        const account = this.client.getUserAccount()!;
        return account.orders.filter((order) => order.orderId > 0);
    }

    async place_order(order: OrderParams): Promise<TransactionSignature> {
        order.baseAssetAmount = new BN(order.baseAssetAmount, 16);
        order.price = new BN(order.price, 16);
        return await this.client.placeOrders([order]);
    }

    async cancel_order(params: CancelOrderParams): Promise<TransactionSignature> {
        console.log('Cancelling Order:', params)
        if (params.orderId != null) {
            return await this.client.cancelOrder(params.orderId);
        } else if (params.orderUserId != null) {
            return await this.client.cancelOrderByUserId(params.orderUserId);
        } else if (params.marketType != null && params.marketIndex != null) {
            return await this.client.cancelOrders(params.marketType, params.marketIndex);
        } else {
            throw new Error('Invalid params');
        }
    }

    async get_positions(): Promise<UserPositions> {
        await this.client.forceGetUserAccount()

        const userAccount = this.client.getUserAccount()!;
        const tokenAmounts = []
        for (const market of this.client.getSpotMarketAccounts()) {
            const amount = this.client.getTokenAmount(market.marketIndex)
            if (amount != 0) {
                tokenAmounts.push({
                    tokenIndex: market.marketIndex,
                    tokenAmount: amount
                });
            }
        }
        const spotPositions = userAccount.spotPositions.filter((position) => position.scaledBalance != 0);
        const perpPositions = userAccount.perpPositions.filter((position) => position.baseAssetAmount != 0);
        return {tokenAmounts, spotPositions, perpPositions};
    }

    async call(call: string, params: object | null): Promise<any> {
        switch (call) {
            case "get_orders":
                return await this.get_orders();
            case "place_order":
                return await this.place_order(params as OrderParams);
            case "cancel_order":
                return await this.cancel_order(params as CancelOrderParams);
            case "get_positions":
                return await this.get_positions();
            default:
                throw new Error(`Unknown function: ${call}`);
        }
    }
}


export type CancelOrderParams = {
    orderId?: number;
    orderUserId?: number;
    marketType?: MarketType;
    marketIndex?: number;
};
export type UserTokenAmount = {
    tokenIndex: number;
    tokenAmount: BN;
};


export type UserPositions = {
    tokenAmounts: UserTokenAmount[];
    spotPositions: SpotPosition[];
    perpPositions: PerpPosition[];
};


async function main() {
    const drift = new Drift();
    await drift.init();

    while (true) {
        const {spotPositions, perpPositions} = await drift.get_positions();
        console.log('Spot Positions:', spotPositions);
        console.log('Perp Positions:', perpPositions);
        const orders = await drift.get_orders();
        console.log('Orders:', orders);

        for (const order of orders) {
            const result = await drift.cancel_order({orderId: order.orderId});
            console.log('Cancel Order Result:', result);
        }
        await new Promise(resolve => setTimeout(resolve, 1000));
    }

}


if (require.main === module) {
    main().catch((error) => {
        console.error("caught", error)
        process.exit(-1)
    });
}
