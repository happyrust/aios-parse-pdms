## 自定义类型
### 说明
自定义类型除了有两个显示属性和base_type(基于某个类型)不同外，基本都一样
### 创建
在元件库 Lexicon 模块,创建 UDETWL->UDETGR->UDET，User-Defined name就是自定义类型的类型名
### 区别
自定义类型中有两个属性能判断是否为自定义类型：<br>
    1.TYPEX:TYPEX的数据就是对应自定义类型的ukey，找到ukey就能找到User-Defined name
    2.UDTYPE: 同样也是ukey，取值和TYPEX一样就行
### 测试方法
需要自己在e3d中创立该类型
aios-parse-pdms 中 tets_17496_161418_udtype()