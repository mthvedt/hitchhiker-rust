package com.thunderhead.core;

/**
 * A DbCreationContext is how you create databases. Fiat Lux!
 */
public interface DbCreationContext {
    void createOrOpen(Class<DatastoreFactory> factoryClass, ByteString dbkey) throws DbCreationException;

    void open(Class<DatastoreFactory> factoryClass, ByteString dbname) throws DbCreationException;

    void createOrOpen(Class<DatastoreFactory> factoryClass, String dbname) throws DbCreationException;

    void open(Class<DatastoreFactory> factoryClass, String dbname) throws DbCreationException;

    // TODO: name
    void destroy(ByteString dbname) throws DbCreationException;

    // TODO: name
    void destroy(String dbname) throws DbCreationException;

    void fork(DbCreator creator, ByteString subname);

    void fork(DbCreator creator, String subname);
}
