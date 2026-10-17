//! The textual atlas. Every page is plain data so that copy can be edited
//! without touching the renderer or the interaction code.

use crate::scenes::SceneId;

/// A single figure in the margin of a page.
#[derive(Clone, Copy, Debug)]
pub struct Stat {
    pub value: &'static str,
    pub label: &'static str,
    pub note: &'static str,
}

/// One full-screen chapter.
#[derive(Clone, Copy, Debug)]
pub struct Page {
    pub scene: SceneId,
    /// Chapter number used by the rail, e.g. `01`.
    pub number: &'static str,
    /// Short Chinese chapter name, e.g. `恒星`.
    pub chapter: &'static str,
    /// Small English eyebrow above the title.
    pub kicker: &'static str,
    pub title: &'static str,
    pub title_en: &'static str,
    pub lead: &'static str,
    pub paragraphs: &'static [&'static str],
    pub stats: &'static [Stat],
    /// Display-space accent used by the interface, not by the shader.
    pub accent: [u8; 3],
    pub cta: Option<&'static str>,
}

const fn stat(value: &'static str, label: &'static str, note: &'static str) -> Stat {
    Stat { value, label, note }
}

pub const PAGES: [Page; 8] = [
    Page {
        scene: SceneId::Prologue,
        number: "00",
        chapter: "序章",
        kicker: "A FIELD GUIDE TO THE DEEP SKY",
        title: "深空图鉴",
        title_en: "DEEP SPACE ATLAS",
        lead: "把目光交给黑暗，让光自己说话。",
        paragraphs: &[
            "这是一部关于深空的视觉图鉴。它从一颗普通的恒星出发，穿过行星的轨道、恒星诞生的云气、旋涡星系的尘埃，最终抵达类星体与黑洞的边界。",
            "所有背景都由实时程序生成：没有照片，没有贴图，只有数学、噪声与光。每一次滚动，都是一次穿越。",
            "建议在安静的环境里，用全屏阅读。",
        ],
        stats: &[
            stat("08", "章节", "CHAPTERS"),
            stat("13.8 Gyr", "宇宙年龄", "COSMIC AGE"),
            stat("930 亿 ly", "可观测直径", "OBSERVABLE"),
            stat("10^80", "原子数量级", "ATOMS, EST."),
        ],
        accent: [136, 168, 255],
        cta: Some("进入图鉴"),
    },
    Page {
        scene: SceneId::Star,
        number: "01",
        chapter: "恒星",
        kicker: "STELLAR FURNACE",
        title: "一颗恒星",
        title_en: "THE STELLAR FURNACE",
        lead: "在引力的中心，氢开始燃烧。",
        paragraphs: &[
            "恒星是宇宙最先点亮的灯。当一团冷气体在自身引力下坍缩，核心温度升到约一千万开尔文，氢核聚变被点燃，一颗恒星便诞生了。",
            "我们的太阳是一颗 G2V 型主序星，表面温度约 5,772 K，核心温度约 1,570 万 K。它每秒把约 6 亿吨氢聚变为氦，其中约 400 万吨质量转化为能量。",
            "光从核心逃到表面，平均要花上十万年；而从表面到地球，只需要八分二十秒。",
        ],
        stats: &[
            stat("5,772 K", "表面温度", "PHOTOSPHERE"),
            stat("1.57×10^7 K", "核心温度", "CORE"),
            stat("G2V", "光谱型", "SPECTRAL TYPE"),
            stat("4.6 Gyr", "年龄", "AGE"),
        ],
        accent: [255, 184, 116],
        cta: None,
    },
    Page {
        scene: SceneId::System,
        number: "02",
        chapter: "行星系",
        kicker: "ORBITAL ARCHITECTURE",
        title: "行星的轨道",
        title_en: "AN ORBITAL ARCHITECTURE",
        lead: "八颗行星，共享同一束光。",
        paragraphs: &[
            "太阳系诞生于约 46 亿年前的一片分子云。尘埃在盘中碰撞、黏结，最终长成八颗行星、数百颗卫星与无数小天体。",
            "行星以不同的速度绕太阳运行：水星只需 88 天，海王星则要 165 年。轨道并非完美的圆，而是略微拉长的椭圆。",
            "在太阳系之外，人类已确认超过 5,000 颗系外行星。每一颗，都是一条新的轨道。",
        ],
        stats: &[
            stat("8", "行星", "PLANETS"),
            stat("4.6 Gyr", "形成时间", "FORMATION"),
            stat("29.8 km/s", "地球公转速度", "ORBITAL SPEED"),
            stat("5,000+", "已知系外行星", "EXOPLANETS"),
        ],
        accent: [148, 194, 255],
        cta: None,
    },
    Page {
        scene: SceneId::Nebula,
        number: "03",
        chapter: "星云",
        kicker: "MOLECULAR CLOUD",
        title: "星云的摇篮",
        title_en: "THE STELLAR NURSERY",
        lead: "在接近绝对零度的黑暗里，新的光正在酝酿。",
        paragraphs: &[
            "星云是星际气体与尘埃的云。分子云的温度可以低到 10 K，密度却足以让氢气结成分子，也让引力找到工作的支点。",
            "当云核坍缩，中心温度与压力上升，第一缕星光穿透尘埃。周围的气体被电离，形成发光的发射星云；未被打扰的部分，则继续沉默地孕育。",
            "猎户座大星云距离我们约 1,344 光年，是肉眼可见的恒星育婴室之一。",
        ],
        stats: &[
            stat("~10 K", "分子云温度", "MOLECULAR CLOUD"),
            stat("数十 ly", "典型尺度", "TYPICAL SCALE"),
            stat("H2 / He", "主要成分", "COMPOSITION"),
            stat("1,344 ly", "猎户座大星云", "ORION NEBULA"),
        ],
        accent: [206, 148, 220],
        cta: None,
    },
    Page {
        scene: SceneId::Galaxy,
        number: "04",
        chapter: "星系",
        kicker: "SPIRAL ISLAND",
        title: "旋涡星系",
        title_en: "AN ISLAND OF STARS",
        lead: "一千亿颗太阳，在同一片引力里旋转。",
        paragraphs: &[
            "银河系是一个棒旋星系，直径约 10 万光年，包含 1,000 亿到 4,000 亿颗恒星。太阳位于猎户臂，距离银心约 2.6 万光年。",
            "星系并非静止的盘。恒星与气体沿旋臂运行，密度波压缩气体，触发新的恒星形成，于是旋臂在黑暗中显出轮廓。",
            "太阳绕银心一周约需 2.3 亿年。上一次它处在今天的位置时，恐龙刚刚出现。",
        ],
        stats: &[
            stat("~100,000 ly", "直径", "DIAMETER"),
            stat("1,000–4,000 亿", "恒星数量", "STARS"),
            stat("~230 Myr", "太阳公转周期", "GALACTIC YEAR"),
            stat("26,000 ly", "太阳距银心", "SUN–CENTER"),
        ],
        accent: [160, 184, 255],
        cta: None,
    },
    Page {
        scene: SceneId::Quasar,
        number: "05",
        chapter: "类星体",
        kicker: "ACTIVE GALACTIC NUCLEUS",
        title: "类星体",
        title_en: "THE QUASAR",
        lead: "一个星系的心脏，比整个星系更亮。",
        paragraphs: &[
            "类星体是活跃星系核最明亮的一类。中心超大质量黑洞吞噬气体，吸积盘在摩擦中升温，释放出从射电到伽马射线的全波段辐射。",
            "最明亮的类星体光度可达 10^46–10^48 erg/s，超过整个银河系数千倍。它们的光来自一个只有太阳系大小、却承载数百万到数十亿倍太阳质量黑洞的区域。",
            "因为光需要时间旅行，我们看到的类星体大多处于宇宙早期。它们像灯塔，标记着星系成长的黎明。",
        ],
        stats: &[
            stat("10^46–10^48", "光度 / erg/s", "LUMINOSITY"),
            stat("> 0.1", "红移", "REDSHIFT"),
            stat("10^6–10^9 M☉", "黑洞质量", "BLACK HOLE"),
            stat("光日", "吸积盘尺度", "DISK SCALE"),
        ],
        accent: [255, 198, 126],
        cta: None,
    },
    Page {
        scene: SceneId::BlackHole,
        number: "06",
        chapter: "黑洞",
        kicker: "EVENT HORIZON",
        title: "事件视界",
        title_en: "THE EVENT HORIZON",
        lead: "光可以绕行，但无法回头。",
        paragraphs: &[
            "黑洞是时空弯曲到极致的区域。事件视界之内，逃逸速度超过光速；外部的观察者只能看到一层漆黑的剪影，以及被引力弯曲的光。",
            "银河系中心的人马座 A* 质量约 430 万倍太阳质量，事件视界半径约 1,270 万公里。它周围的气体以接近光速运行，发出明亮的射电与 X 射线。",
            "2019 年，事件视界望远镜发布了 M87* 的第一张照片：一个明亮的环，围着一片比黑夜更黑的阴影。",
        ],
        stats: &[
            stat("4.3×10^6 M☉", "人马座 A*", "SGR A*"),
            stat("~1.3×10^7 km", "史瓦西半径", "Rs"),
            stat("~2.6 Rs", "光子环", "PHOTON RING"),
            stat("2019", "首张照片", "M87*"),
        ],
        accent: [255, 176, 106],
        cta: None,
    },
    Page {
        scene: SceneId::Epilogue,
        number: "07",
        chapter: "终章",
        kicker: "AFTERWORD",
        title: "回到光里",
        title_en: "RETURN TO THE LIGHT",
        lead: "我们由星尘构成，也终将望向星尘。",
        paragraphs: &[
            "可观测宇宙的直径约 930 亿光年，包含约 2 万亿个星系。每一个星系，都是一座由恒星组成的岛。",
            "你身体里的碳、氧、铁，几乎都来自早已死去的恒星。它们在超新星爆发中把元素撒向太空，新的行星与生命由此成为可能。",
            "图鉴到这里结束，但天空没有尽头。",
        ],
        stats: &[
            stat("930 亿 ly", "可观测直径", "OBSERVABLE"),
            stat("~2 万亿", "星系数量", "GALAXIES, EST."),
            stat("13.8 Gyr", "宇宙年龄", "COSMIC AGE"),
            stat("∞", "未知", "UNKNOWN"),
        ],
        accent: [204, 216, 244],
        cta: None,
    },
];

/// The number of pages, kept in one place for the rail and the input clamp.
pub const PAGE_COUNT: usize = PAGES.len();

/// Safe access; the app only ever holds indices produced by navigation.
pub fn page(index: usize) -> &'static Page {
    &PAGES[index.min(PAGE_COUNT - 1)]
}
