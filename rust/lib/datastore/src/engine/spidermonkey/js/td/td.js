// db.js
//
// The master Javascript store.

var Td = (function() {
  var Td = {};

  Td.Table = function(name) {
    return {
      store: {
        type: "table",
      },
      name: "name",
      mode: "create_or_set",
    }
  };

  // TODO is this even right?
  Td.Store = function() {
    this.store = {
      type: "multi",
    };

    this.substores = {};

    this.add_store = function(name) {
      // TODO: validation
      this.substores[name] = Td.Store(name);
    }

    this.add_table = function(name) {
      // TODO: validation
      this.substores[name] = Td.Table(name);
    }
  }

  return Td;
})();
