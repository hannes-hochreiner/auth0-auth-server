import {getRelevantRolesFromVerbPath, getIntersection} from '../bld/utils';

describe("utils.getRelevantRolesFromVerbPath", function() {
  it("can find the relevant roles for a path", function() {
    const conf = {
      '/': {
        'GET': ['read'],
        'PUT': ['write']
      },
      '/test1': {
        'POST': ['post']
      },
      '/test1/test2': {
        'OPTIONS': ['options']
      }
    };

    expect(getRelevantRolesFromVerbPath(conf, '/test', 'GET')).toEqual(['read']);
    expect(getRelevantRolesFromVerbPath(conf, '/', 'POST')).toEqual([]);
    expect(getRelevantRolesFromVerbPath(conf, '/', 'PUT')).toEqual(['write']);
    expect(getRelevantRolesFromVerbPath(conf, '/test1', 'POST')).toEqual(['post']);
    expect(getRelevantRolesFromVerbPath(conf, '/test1/test2', 'OPTIONS')).toEqual(['options']);
  });
});

describe("utils.getIntersection", function() {
  it("can find the intersection of two arrays", function() {
    expect(getIntersection(['1', '2'], ['2', '3'])).toEqual(['2']);
  });
});
