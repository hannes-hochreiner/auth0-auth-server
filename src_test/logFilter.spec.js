import {LogFilter} from '../bld/logFilter';
import {SequenceSpy} from './sequenceSpy';

describe("LogFilter", function() {
  it("will not log messages below the selected filter", function() {
    let lf = new LogFilter(new SequenceSpy([
      {name: 'log', args: ['testLog']},
      {name: 'error', args: ['testError']},
      {name: 'warn', args: ['testWarn']}
    ]), 'warn');

    lf.log('testLog');
    lf.error('testError');
    lf.warn('testWarn');
    lf.info('testInfo');
    lf.debug('testDebug');
  });
});
