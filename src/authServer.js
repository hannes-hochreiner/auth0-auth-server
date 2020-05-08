export class AuthServer {
  constructor(http, verify, getRelevantRolesFromVerbPath, getIntersection, logger) {
    this._http = http;
    this._verify = verify;
    this._getRelevantRolesFromVerbPath = getRelevantRolesFromVerbPath;
    this._getIntersection = getIntersection;
    this._logger = logger;
  }

  async init(conf, headerNames) {
    this._conf = conf;
    this._headerNames = headerNames || {};
    this._server = new this._http.Server();
    this._server.on('request', this._requestHandler.bind(this));
    this._server.listen(8888);
    this._logger.info(`authentication server listening at port 8888`);
  }

  async _requestHandler(request, response) {
    try {
      this._logger.debug(request);
      let verb = request.headers[this._headerNames.method || 'x-original-method'];
      let path = request.headers[this._headerNames.uri || 'x-original-uri'];
      let token = request.headers.authorization.split(' ')[1];
      let roles = this._getRelevantRolesFromVerbPath(this._conf, path, verb);
      let tokenDecoded = await this._verify(token);
      this._logger.debug(tokenDecoded);
      let scopes = tokenDecoded.scope.split(' ');

      if (typeof tokenDecoded.permissions !== 'undefined') {
        scopes = scopes.concat(tokenDecoded.permissions);
      }

      let inter = this._getIntersection(roles, scopes);
      this._logger.debug(`intersection of roles and scopes: ${JSON.stringify(inter)}`);

      if (inter.length > 0) {
        response.statusCode = 200;
        // response.setHeader("x-id", result.id);
        response.setHeader(this._headerNames.groups || 'x-groups', inter.join(','));
      } else {
        response.statusCode = 403;
      }
  
      this._logger.info(`${(new Date()).toISOString()}\t${response.statusCode}\t${verb}\t${path}`);
    } catch (error) {
      response.statusCode = 500;
      this._logger.error(error);
    } finally {
      response.end();
    }
  }
}
