import {Verifier} from '../bld/verifier';

describe("Verifier", function() {
  it("can be constructed", function() {
    expect(new Verifier()).not.toThrow;
  });

  it("can be initialized", function() {
    let v = new Verifier(jwksMock, {});

    v.init('jwksUri', 'audience', 'issuer', ['algorithms']);

    expect(v._audience).toEqual('audience');
    expect(v._issuer).toEqual('issuer');
    expect(v._algorithms).toEqual(['algorithms']);
  });

  it("can verify a token", async function() {
    let v = new Verifier(jwksMock, new JwtMock());

    v.init('jwksUri', 'audience', 'issuer', ['algorithms']);
    expect(await v.verify('token')).toEqual('decoded');
  });
});

function jwksMock(options) {
  expect(options.jwksUri).toEqual('jwksUri');
  return new ClientMock();
}

class ClientMock {
  getSigningKey(kid, callback) {
    expect(kid).toEqual('kid');
    callback(undefined, {publicKey: 'signingKey'});
  }
}

class JwtMock {
  verify(token, keyFun, options, callback) {
    expect(token).toEqual('token');
    expect(options).toEqual({
      audience: 'audience',
      issuer: 'issuer',
      algorithms: ['algorithms']
    });
    keyFun({kid: 'kid'}, this.keyCallback.bind(this, callback));
  }

  keyCallback(verifyCallback, error, signingKey) {
    expect(error).toBeUndefined;
    expect(signingKey).toEqual('signingKey');
    verifyCallback(undefined, 'decoded');
  }
}
