const assert = require("assert");
const anchor = require("@project-serum/anchor");
//const serum = require("@project-serum/serum");
const serum = require("/home/armaniferrante/Documents/code/src/github.com/project-serum/serum-ts/packages/serum");
const { BN } = anchor;
const { Transaction, TransactionInstruction } = anchor.web3;
const { DexInstructions } = serum;
const { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } = anchor.web3;
const { initMarket } = require("./utils");

const DEX_PID = new PublicKey("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");

describe("permissioned-markets", () => {
  // Anchor client setup.
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.PermissionedMarkets;

  // Global DEX accounts and clients shared accross all tests.
  let marketClient, usdcAccount;
  let openOrders, openOrdersBump, openOrdersInitAuthority, openOrdersBumpinit;

  it("BOILERPLATE: Initializes an orderbook", async () => {
    const { marketA, godUsdc } = await initMarket({ provider });
    marketClient = marketA;
    marketClient._programId = program.programId;
    usdcAccount = godUsdc;
  });

  it("BOILERPLATE: Calculates open orders addresses", async () => {
    const [_openOrders, bump] = await PublicKey.findProgramAddress(
      [
        anchor.utils.bytes.utf8.encode("open-orders"),
        marketClient.address.toBuffer(),
        program.provider.wallet.publicKey.toBuffer(),
      ],
      program.programId
    );
    const [
      _openOrdersInitAuthority,
      bumpInit,
    ] = await PublicKey.findProgramAddress(
      [
        anchor.utils.bytes.utf8.encode("open-orders-init"),
        marketClient.address.toBuffer(),
      ],
      program.programId
    );

    // Save global variables re-used across tests.
    openOrders = _openOrders;
    openOrdersBump = bump;
    openOrdersInitAuthority = _openOrdersInitAuthority;
    openOrdersBumpInit = bumpInit;
  });

  it("Creates an open orders account", async () => {
    await program.rpc.initAccount(openOrdersBump, openOrdersBumpInit, {
      accounts: {
        openOrdersInitAuthority,
        openOrders,
        authority: program.provider.wallet.publicKey,
        market: marketClient.address,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
        dexProgram: DEX_PID,
      },
    });

    const account = await provider.connection.getAccountInfo(openOrders);
    assert.ok(account.owner.toString() === DEX_PID.toString());
  });

  it("Posts a bid on the orderbook", async () => {
    const tx = new Transaction();
    tx.add(
      serumProxy(
        marketClient.makePlaceOrderInstruction(program.provider.connection, {
          owner: program.provider.wallet.publicKey,
          payer: usdcAccount,
          side: "buy",
          price: 1.1234,
          size: 1234,
          orderType: "limit",
          clientId: new BN(999),
          openOrdersAddressKey: openOrders,
          selfTradeBehavior: "abortTransaction",
        })
      )
    );
    await provider.send(tx);
  });

  it("Cancels a bid on the orderbook", async () => {
    // todo
  });

  it("Settles funds on the orderbook", async () => {
    // todo
  });

  it("Closes an open orders account", async () => {
    // Given.
    const beforeAccount = await program.provider.connection.getAccountInfo(
      program.provider.wallet.publicKey
    );

    // When.
    const tx = new Transaction();
    tx.add(
      serumProxy(
        DexInstructions.closeOpenOrders({
          market: marketClient._decoded.ownAddress,
          openOrders,
          owner: program.provider.wallet.publicKey,
          solWallet: program.provider.wallet.publicKey,
          programId: program.programId,
        })
      )
    );
    await provider.send(tx);

    // Then.
    const afterAccount = await program.provider.connection.getAccountInfo(
      program.provider.wallet.publicKey
    );
    const closedAccount = await program.provider.connection.getAccountInfo(
      openOrders
    );

    assert.ok(23352768 === afterAccount.lamports - beforeAccount.lamports);
    assert.ok(closedAccount === null);
  });
});

// Adds the serum dex account to the instruction so that proxies can
// relay (CPI requires the executable account).
//
// TODO: we should add flag in the dex client that says if a proxy is being
//       used, and if so, do this automatically.
function serumProxy(ix) {
  ix.keys = [
    { pubkey: DEX_PID, isWritable: false, isSigner: false },
    ...ix.keys,
  ];
  return ix;
}
