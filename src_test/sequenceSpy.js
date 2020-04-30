export class SequenceSpy {
  constructor(config) {
    this._config = config;

    for (let idx in this._config) {
      let funName = this._config[idx].name;
      this[funName] = this.fun.bind(this, funName);
    }
  }

  fun(funName, ...args) {
    // console.log(funName, args);
    let step = this._config.shift();

    expect(funName).toEqual(step.name);

    if (typeof step.args !== 'undefined') {
      expect(args).toEqual(step.args);
    }

    if (typeof step.return !== 'undefined') {
      return step.return;
    }
  }
}
