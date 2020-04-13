function _sortDescendingByLength(elem1, elem2) {
  if (elem1.length > elem2.length) {
    return -1;
  }

  if (elem1.length < elem2.length) {
    return 1;
  }

  return 0;
}

export function getRelevantRolesFromVerbPath(conf, path, verb) {
  let paths = Object.keys(conf).filter(elem => {
    return path.startsWith(elem);
  });

  if (paths.length == 0) {
    return [];
  }

  return conf[paths.sort(_sortDescendingByLength)[0]][verb.toUpperCase()] || [];
}

export function getIntersection(array1, array2) {
  return array1.filter(elem => {
    return array2.includes(elem);
  });
}
