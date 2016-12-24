// get_store.js
//
// A system function that gets the user-defined store from a schema.

({
  function(userobj, tdobj) {
    // if userobj.build_store {
      return userobj.build_store(new tdobj.Store());
      // TODO
    // } else if obj.create_store {
    // } else {
    //   throw "Cannot build schema; please define build_schema or create_schema"
    // }
  },
})
