# The engine's Kotlin bindings (UniFFI) reach the Rust library through JNA, which finds classes,
# fields and methods by name at run time. Keep them as they are.
-keep class com.sun.jna.** { *; }
-keep class * extends com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keep class io.github.kimzam30.nhakobeam.core.** { *; }
-dontwarn java.awt.**

# Line numbers in crash reports.
-keepattributes SourceFile,LineNumberTable
-renamesourcefileattribute SourceFile
