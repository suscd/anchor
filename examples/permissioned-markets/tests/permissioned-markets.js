const assert = require("assert");
const anchor = require("@project-serum/anchor");
const serum = require("@project-serum/serum");
const { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } = anchor.web3;
const { initMarket } = require("./utils");

const DEX_PID = new PublicKey("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin");

describe("permissioned-markets", () => {
  const provider = anchor.Provider.env();
  anchor.setProvider(provider);

  let ORDERBOOK_ENV;

  it("Initializes an orderbook", async () => {
    ORDERBOOK_ENV = await initMarket({ provider });
  });

  it("Is initialized!", async () => {
    console.log("env", ORDERBOOK_ENV);

    const program = anchor.workspace.PermissionedMarkets;
    const [openOrders, bump] = await PublicKey.findProgramAddress(
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
    console.log(account);
    assert.ok(account.owner.toString() === DEX_PID.toString());
  });
});
