# Changelog

## 2026-01-15
- 修复隐式 DOUBLE/Vec3/DOUBLEVEC 的字序解析，避免 ANGL 等角度值被误读为天文数字。
- 显式表达式补齐 0x65 ValueExpression 数值解析，数字表达式可落入可数值读取的类型。
