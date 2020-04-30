import {AuthServer} from '../bld/authServer';

describe("AuthServer", function() {
  it("can be constructed", function() {
    expect(new AuthServer()).not.toThrow;
  });

  it("can be initialized", function() {
    let as = new AuthServer({Server: ServerMock}, null, null, null, new LoggerMute());

    as.init('conf');

    expect(as._conf).toEqual('conf');
    expect(as._server instanceof ServerMock).toBeTrue;
  });

  it("can handle authorized requests from scope", async function() {
    let as = new AuthServer({Server: ServerMock}, verify, rolesFromPath, intersection, new LoggerMute());

    as.init('conf');

    let resp = new ResponseMock();

    await as._requestHandler({
      headers: {
        'x-original-method': 'verb',
        'x-original-uri': 'path',
        authorization: 'auth token'
      }
    }, resp);

    expect(resp._headers).toEqual({
      'x-groups': 'inter1,inter2'
    });
    expect(resp.statusCode).toEqual(200);
    expect(resp._endWasCalled).toBeTrue;
  });

  it("can handle authorized requests from permissions", async function() {
    let as = new AuthServer({Server: ServerMock}, verifyPermissions, rolesFromPath, intersectionPermissions, new LoggerMute());

    as.init('conf');

    let resp = new ResponseMock();

    await as._requestHandler({
      headers: {
        'x-original-method': 'verb',
        'x-original-uri': 'path',
        authorization: 'auth token'
      }
    }, resp);

    expect(resp._headers).toEqual({
      'x-groups': 'inter1'
    });
    expect(resp.statusCode).toEqual(200);
    expect(resp._endWasCalled).toBeTrue;
  });

  it("can handle unauthorized requests", async function() {
    let as = new AuthServer({Server: ServerMock}, verify, rolesFromPath, intersectionEmpty, new LoggerMute());

    as.init('conf');

    let resp = new ResponseMock();

    await as._requestHandler({
      headers: {
        'x-original-method': 'verb',
        'x-original-uri': 'path',
        authorization: 'auth token'
      }
    }, resp);

    expect(resp._headers).toEqual({});
    expect(resp.statusCode).toEqual(403);
    expect(resp._endWasCalled).toBeTrue;
  });

  it("can handle failed requests", async function() {
    let as = new AuthServer({Server: ServerMock}, verifyFailed, rolesFromPath, intersection, new LoggerMute());

    as.init('conf');

    let resp = new ResponseMock();

    await as._requestHandler({
      headers: {
        'x-original-method': 'verb',
        'x-original-uri': 'path',
        authorization: 'auth token'
      }
    }, resp);

    expect(resp._headers).toEqual({});
    expect(resp.statusCode).toEqual(500);
    expect(resp._endWasCalled).toBeTrue;
  });
});

function rolesFromPath(conf, path, verb) {
  expect(conf).toEqual('conf');
  expect(path).toEqual('path');
  expect(verb).toEqual('verb');

  return 'roles';
}

function verify(token) {
  expect(token).toEqual('token');

  return {scope: 'scope1 scope2'};
}

function verifyPermissions(token) {
  expect(token).toEqual('token');

  return {
    scope: 'scope1 scope2',
    permissions: [
      'permission1'
    ]
  };
}

function verifyFailed(token) {
  expect(token).toEqual('token');

  throw new Error();
}

function intersection(array1, array2) {
  expect(array1).toEqual('roles');
  expect(array2).toEqual(['scope1', 'scope2']);

  return ['inter1', 'inter2'];
}

function intersectionPermissions(array1, array2) {
  expect(array1).toEqual('roles');
  expect(array2).toEqual(['scope1', 'scope2', 'permission1']);

  return ['inter1'];
}

function intersectionEmpty(array1, array2) {
  expect(array1).toEqual('roles');
  expect(array2).toEqual(['scope1', 'scope2']);

  return [];
}

class ResponseMock {
  constructor() {
    this._endWasCalled = false;
    this._headers = {};
  }

  end() {
    this._endWasCalled = true;
  }

  setHeader(header, value) {
    this._headers[header] = value;
  }
}

class ServerMock {
  on(event, callback) {
    expect(event).toEqual('request');
    expect(typeof callback).toEqual('function');
  }

  listen(port) {
    expect(port).toEqual(8888);
  }
}

class LoggerMute {
  constructor() {
    for (let ll of ['debug', 'info', 'warn', 'error', 'log']) {
      this[ll] = () => {};
    }
  }
}
