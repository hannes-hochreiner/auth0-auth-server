import {default as commander} from 'commander';
import {default as http} from 'http';
import {default as jwks} from 'jwks-rsa';
import {default as jwt} from 'jsonwebtoken';
import {readFile} from './fs';
import {getRelevantRolesFromVerbPath, getIntersection} from './utils';
import {AuthServer} from './authServer';
import {Verifier} from './verifier';
import { LogFilter } from './logFilter';

commander.option('-c, --configuration [path]', 'path of the configuration file').parse(process.argv);

async function init() {
  try {
    const conf = JSON.parse(await readFile(commander.configuration));
    const verifier = new Verifier(jwks, jwt);
    const log = new LogFilter(console, conf.logLevel || 'warning');

    verifier.init(conf.jwksUri, conf.audience, conf.issuer, conf.algorithms);

    const server = new AuthServer(http, verifier.verify.bind(verifier), getRelevantRolesFromVerbPath, getIntersection, log);

    server.init(conf.auth);
  } catch(error) {
    console.log(error);
  }
}

init();
