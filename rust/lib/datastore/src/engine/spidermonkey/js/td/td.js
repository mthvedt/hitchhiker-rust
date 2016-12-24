// td.js
//
// Td.js is a special file, in that it is the master file for

// TODO: make sure everything optimizes. The following should optimize:
// * Access to each function slot (obj.Table, obj.Store...) should be single dispatch
// (at least Spidermonkey GetProp_DefiniteSlot but preferably GetProp_InferredConstant)
// * Access to each function slot, when declared at the top level, should optimize to a constant
// (Spidermonkey GetProp_InferredConstant)
// * The functions themselves

({
  Table(name) {
    return {
      store: {
        type: "table",
      },
      name: "name",
      mode: "create_or_set",
    }
  },

  // TODO is this even right?
  Store() {
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
})
