export class Verifier {
  constructor(jwks, jwt) {
    this._jwks = jwks;
    this._jwt = jwt;
  }

  init(jwksUri, audience, issuer, algorithms) {
    this._audience = audience;
    this._issuer = issuer;
    this._algorithms = algorithms;
    this._client = this._jwks({
      // cache: true, // Default Value
      // cacheMaxEntries: 5, // Default value
      // cacheMaxAge: ms('10m'), // Default value
      rateLimit: true,
      // jwksRequestsPerMinute: 10, // Default value
      jwksUri: jwksUri
    });
  }

  _getKey(header, callback){
    this._client.getSigningKey(header.kid, function(err, key) {
      var signingKey = key.publicKey || key.rsaPublicKey;
      callback(err, signingKey);
    });
  }

  verify(token) {
    return new Promise((resolve, reject) => {
      this._jwt.verify(token, this._getKey.bind(this), {
        audience: this._audience,
        issuer: this._issuer,
        algorithms: this._algorithms
      }, (err, decoded) => {
        if (err) {
          reject(err);
        }
  
        resolve(decoded);
      });
    });
  }
}
