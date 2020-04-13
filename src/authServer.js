export class AuthServer {
  constructor(http, verify, getRelevantRolesFromVerbPath, getIntersection) {
    this._http = http;
    this._verify = verify;
    this._getRelevantRolesFromVerbPath = getRelevantRolesFromVerbPath;
    this._getIntersection = getIntersection;
  }

  async init(conf) {
    this._conf = conf;
    this._server = new this._http.Server();
    this._server.on('request', this._requestHandler.bind(this));
    this._server.listen(8888);
  }

  async _requestHandler(request, response) {
    try {
      let verb = request.headers['x-original-method'];
      let path = request.headers['x-original-uri'];
      let token = request.headers.authorization.split(' ')[1];
      let roles = this._getRelevantRolesFromVerbPath(this._conf, path, verb);
      let tokenDecoded = await this._verify(token);
      let scopes = tokenDecoded.scope.split(' ');
      let inter = this._getIntersection(roles, scopes);

      if (inter.length > 0) {
        response.statusCode = 200;
        // response.setHeader("x-id", result.id);
        response.setHeader("x-groups", inter.join(','));
      } else {
        response.statusCode = 403;
      }
  
      console.log(`${(new Date()).toISOString()}\t${response.statusCode}\t${verb}\t${path}`);
    } catch (error) {
      response.statusCode = 500;
      console.dir(error);
    } finally {
      response.end();
    }
  }
}
