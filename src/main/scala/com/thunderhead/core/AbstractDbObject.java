//package com.thunderhead.core;
//
//import java.util.Arrays;
//
///**
// * Created by Mike on 7/3/16.
// */
//class AbstractDbObject {
//
//    @Override
//    public int compareTo(T t) {
//        for (int i = 0; ; i++) {
//            if (i >= bytes.length) {
//                if (i >= t.bytes.length) {
//                    return 0;
//                } else {
//                    return -1; // dictionary ordering
//                }
//            } else {
//                if (i >= t.bytes.length) {
//                    return 1; // dictionary ordering
//                } else {
//                    byte a = bytes[i];
//                    byte b = t.bytes[i];
//                    int r = (int)a - (int)b;
//
//                    if (r != 0) {
//                        return r;
//                    }
//                }
//            }
//        }
//    }
//
//    @Override
//    public boolean equals(Object o) {
//        if (this == o) {
//            return true;
//        }
//
//        if (o == null || getClass() != o.getClass()) {
//            return false;
//        }
//
//        AbstractByteString that = (AbstractByteString)o;
//        return Arrays.equals(bytes, that.bytes);
//    }
//
//    @Override
//    public int hashCode() {
//        return Arrays.hashCode(bytes);
//    }
//}
