trait JsResult<T> {
}

trait JsLambda {
    fn exec(&self) -> JsResult<()>;
}

trait JsExecutor {
    //fn read(&r: Reader) -> JsResult<JsLambda>;
}

