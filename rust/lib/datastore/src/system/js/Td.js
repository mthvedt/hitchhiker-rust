// db.js
//
// The master Javascript store.

var Td = (function() {
  var Td = {};

  Td.collection = function(name) {
    return {
      store: {
        type: "doc",
      },
      name: "name",
      mode: "create_or_set",
    }
  };

  // TODO is this even right?
  Td.datastore = function() {
    this.store = {
      type: "multi",
    };

    this.substores = {};

    this.add_collection = function(name) {
      // TODO: validation
      this.substores[name] = Td.collection(name);
    }
  }
})();
