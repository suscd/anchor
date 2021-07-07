const assert = require("assert");
const anchor = require("@project-serum/anchor");
const serum = require("@project-serum/serum");
const { Transaction, TransactionInstruction } = anchor.web3;
const { DexInstructions } = serum;
const { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } = anchor.web3;
const { initMarket } = require("./utils");

const DEX_PID = new PublicKey("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");

describe("permissioned-markets", () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.PermissionedMarkets;

  let ORDERBOOK_ENV;
  let openOrders;

  it("Initializes an orderbook", async () => {
    ORDERBOOK_ENV = await initMarket({ provider });
  });

  it("Creates an open orders account", async () => {
    const [_openOrders, bump] = await PublicKey.findProgramAddress(
      [
        anchor.utils.bytes.utf8.encode("open-orders"),
        ORDERBOOK_ENV.marketA.address.toBuffer(),
        program.provider.wallet.publicKey.toBuffer(),
      ],
      program.programId
    );
    const [
      openOrdersInitAuthority,
      bumpInit,
    ] = await PublicKey.findProgramAddress(
      [
        anchor.utils.bytes.utf8.encode("open-orders-init"),
        ORDERBOOK_ENV.marketA.address.toBuffer(),
      ],
      program.programId
    );
    openOrders = _openOrders;

    await program.rpc.initAccount(bump, bumpInit, {
      accounts: {
        openOrdersInitAuthority,
        openOrders,
        authority: program.provider.wallet.publicKey,
        market: ORDERBOOK_ENV.marketA.address,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
        dexProgram: DEX_PID,
      },
    });

    const account = await provider.connection.getAccountInfo(openOrders);
    assert.ok(account.owner.toString() === DEX_PID.toString());
  });

  it("Closes an open orders account", async () => {
    const beforeAccount = await program.provider.connection.getAccountInfo(
      program.provider.wallet.publicKey
    );
    const tx = new Transaction();
    tx.add(
      serumProxy(
        DexInstructions.closeOpenOrders({
          market: ORDERBOOK_ENV.marketA._decoded.ownAddress,
          openOrders,
          owner: program.provider.wallet.publicKey,
          solWallet: program.provider.wallet.publicKey,
          programId: program.programId,
        })
      )
    );
    await program.provider.send(tx);

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
function serumProxy(ix) {
  ix.keys = [
    { pubkey: DEX_PID, isWritable: false, isSigner: false },
    ...ix.keys,
  ];
  return ix;
}
