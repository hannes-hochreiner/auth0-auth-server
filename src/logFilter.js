const logLevels = ['debug', 'info', 'warn', 'error', 'log'];

export class LogFilter {
  constructor(log, logLevel) {
    this._log = log;
    this._logLevels = logLevels.slice(logLevels.findIndex(elem => elem === logLevel));

    for (let ll of logLevels) {
      if (this._logLevels.includes(ll)) {
        this[ll] = (...args) => {this._log[ll].apply(this._log, args)};
      } else {
        this[ll] = () => {};
      }
    }
  }
}
