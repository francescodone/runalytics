import UIKit

// App-authored motion over SF Symbols and the shared runner artwork, not
// UseAnimations assets. Template frames use UITabBarItem's public image API;
// UIKit owns placement, input and accessibility without private item views.
enum NativeRunnerGlyph {
    struct Limb {
        let root: CGPoint
        let joint: CGPoint
        let tip: CGPoint
    }

    struct Pose {
        let head: CGPoint
        let hip: CGPoint
        let shoulder: CGPoint
        let frontLeg: Limb
        let backLeg: Limb
        let frontArm: Limb
        let backArm: Limb
    }

    static func pose(at progress: Double) -> Pose {
        let phase = min(1, max(0, progress))
        let stride = 45 * (1 - cos(phase * .pi * 2))
        let bob = CGFloat(0.2 * pow(sin(phase * .pi * 2), 2))
        let hip = CGPoint(x: 11, y: 14 + bob)
        let shoulder = CGPoint(x: 12, y: 8 + bob)
        func knee(_ phase: Double) -> Double {
            (phase < 0.5 ? -20 : 65) * pow(sin(phase * .pi * 2), 2)
        }
        func rotated(_ vector: CGPoint, by degrees: Double) -> CGPoint {
            let radians = degrees * .pi / 180
            return CGPoint(x: vector.x * cos(radians) - vector.y * sin(radians),
                           y: vector.x * sin(radians) + vector.y * cos(radians))
        }
        func limb(root: CGPoint, upper: CGPoint, lower: CGPoint, angle: Double, bend: Double) -> Limb {
            let thigh = rotated(upper, by: angle)
            let shin = rotated(lower, by: angle + bend)
            let joint = CGPoint(x: root.x + thigh.x, y: root.y + thigh.y)
            return Limb(root: root, joint: joint, tip: CGPoint(x: joint.x + shin.x, y: joint.y + shin.y))
        }
        return Pose(
            head: CGPoint(x: 13.007, y: 5 + bob), hip: hip, shoulder: shoulder,
            frontLeg: limb(root: hip, upper: CGPoint(x: 3.6, y: 2.7), lower: CGPoint(x: 0, y: 4), angle: stride, bend: knee(phase)),
            backLeg: limb(root: hip, upper: CGPoint(x: 3.6, y: 2.7), lower: CGPoint(x: 0, y: 4), angle: 90 - stride, bend: knee((phase + 0.5).truncatingRemainder(dividingBy: 1))),
            frontArm: limb(root: shoulder, upper: CGPoint(x: 2.5, y: 2.7), lower: CGPoint(x: 2.8, y: -1.4), angle: 90 - stride, bend: stride - 90),
            backArm: limb(root: shoulder, upper: CGPoint(x: 2.5, y: 2.7), lower: CGPoint(x: 2.8, y: -1.4), angle: stride, bend: -stride)
        )
    }

    static func draw(in context: CGContext, size: CGSize, progress: Double) {
        let pose = pose(at: progress)
        let scale = min(size.width, size.height) / 24
        context.saveGState()
        defer { context.restoreGState() }
        context.translateBy(x: (size.width - 24 * scale) / 2, y: (size.height - 24 * scale) / 2)
        context.scaleBy(x: scale, y: scale)
        context.setStrokeColor(UIColor.black.cgColor)
        context.setLineWidth(2)
        context.setLineCap(.round)
        context.setLineJoin(.round)
        for limb in [pose.backLeg, pose.frontLeg, pose.backArm, pose.frontArm] {
            context.move(to: limb.root)
            context.addLine(to: limb.joint)
            context.addLine(to: limb.tip)
            context.strokePath()
        }
        context.move(to: pose.hip)
        context.addLine(to: pose.shoulder)
        context.strokePath()
        context.strokeEllipse(in: CGRect(x: pose.head.x - 2, y: pose.head.y - 2, width: 4, height: 4))
    }
}

final class NativeTabIconHost: NSObject {
    enum Artwork: String, CaseIterable {
        case calendar = "calendar"
        case runner = "figure.run"
        case chart = "chart.line.uptrend.xyaxis"
        case trophy = "trophy"
    }

    static let replayDuration: CFTimeInterval = 1.12
    let item: UITabBarItem
    let artwork: Artwork
    private let originalImage: UIImage?
    private let source: UIImage?
    private let renderer: UIGraphicsImageRenderer?
    private var startedAt: CFTimeInterval = 0
    private(set) var displayLink: CADisplayLink?
    var isPlaying: Bool { displayLink != nil }

    private final class DisplayLinkTarget: NSObject {
        weak var host: NativeTabIconHost?
        init(host: NativeTabIconHost) { self.host = host }
        @objc func tick(_ link: CADisplayLink) { host?.tick(link) }
    }

    init(item: UITabBarItem, artwork: Artwork) {
        self.item = item
        self.artwork = artwork
        let source = item.image?.applyingSymbolConfiguration(UIImage.SymbolConfiguration(pointSize: 23, weight: .regular))
        self.source = source
        let renderer = source.map { UIGraphicsImageRenderer(size: $0.size) }
        self.renderer = renderer
        if artwork == .runner, let source = source, let renderer = renderer {
            originalImage = Self.matchMetrics(renderer.image { context in
                NativeRunnerGlyph.draw(in: context.cgContext, size: source.size, progress: 0)
            }, to: source)
            item.image = originalImage
            item.selectedImage = nil
        } else {
            originalImage = item.image
        }
        super.init()
    }

    deinit { cancelPlayback() }

    func replay() {
        cancelPlayback()
        guard source != nil, !UIAccessibility.isReduceMotionEnabled else { return }
        startedAt = CACurrentMediaTime()
        let link = CADisplayLink(target: DisplayLinkTarget(host: self), selector: #selector(DisplayLinkTarget.tick(_:)))
        link.preferredFramesPerSecond = 60
        displayLink = link
        link.add(to: .main, forMode: .common)
    }

    func tick(_ link: CADisplayLink) {
        guard link === displayLink else { return }
        advance(to: CACurrentMediaTime() - startedAt)
    }

    func advance(to elapsed: CFTimeInterval) {
        guard isPlaying else { return }
        guard elapsed < Self.replayDuration, !UIAccessibility.isReduceMotionEnabled,
              let source = source, let renderer = renderer else {
            cancelPlayback()
            return
        }
        let progress = max(0, elapsed / Self.replayDuration)
        let amount = progress < 0.28 ? Self.ease(progress / 0.28) : 1 - Self.ease((progress - 0.28) / 0.72)
        let frame = Self.matchMetrics(renderer.image { context in
            if artwork == .runner {
                // Linear gait time matches the web runner's two alternating steps.
                NativeRunnerGlyph.draw(in: context.cgContext, size: source.size, progress: progress)
                return
            }
            let center = CGPoint(x: source.size.width / 2, y: source.size.height / 2)
            context.cgContext.translateBy(x: center.x, y: center.y)
            switch artwork {
            case .calendar: context.cgContext.translateBy(x: 0, y: -amount)
            case .runner: break
            case .chart: context.cgContext.scaleBy(x: 1 - 0.04 * amount, y: 1 - 0.04 * amount)
            case .trophy: context.cgContext.translateBy(x: 0, y: -1.25 * amount)
            }
            source.draw(at: CGPoint(x: -center.x, y: -center.y))
        }, to: source)
        UIView.performWithoutAnimation {
            item.image = frame
            item.selectedImage = frame
        }
    }

    func cancelPlayback() {
        guard let link = displayLink else { return }
        displayLink = nil
        link.invalidate()
        UIView.performWithoutAnimation {
            item.image = originalImage
            item.selectedImage = nil
        }
    }

    private static func matchMetrics(_ image: UIImage, to source: UIImage) -> UIImage {
        var image = image.withRenderingMode(.alwaysTemplate).withAlignmentRectInsets(source.alignmentRectInsets)
        if let baseline = source.baselineOffsetFromBottom {
            image = image.withBaselineOffset(fromBottom: baseline)
        }
        return image
    }

    // Prism cubic-bezier(.22, 1, .36, 1), evaluated for display-linked frames.
    private static func ease(_ progress: Double) -> CGFloat {
        var low = 0.0
        var high = 1.0
        for _ in 0..<16 {
            let t = (low + high) / 2
            let x = 3 * (1 - t) * (1 - t) * t * 0.22 + 3 * (1 - t) * t * t * 0.36 + t * t * t
            if x < progress { low = t } else { high = t }
        }
        let t = (low + high) / 2
        return CGFloat(1 - pow(1 - t, 3))
    }
}

final class NativeTabBar: UITabBar {
    private(set) var iconHosts: [NativeTabIconHost] = []
    private var observers: [NSObjectProtocol] = []
    private var iconLayout: CGRect = .null
    private var iconSafeArea: UIEdgeInsets = .zero

    override init(frame: CGRect) {
        super.init(frame: frame)
        let notifications: [Notification.Name] = [
            UIAccessibility.reduceMotionStatusDidChangeNotification,
            UIApplication.willResignActiveNotification,
            UIApplication.didEnterBackgroundNotification,
            UIScene.willDeactivateNotification,
            UIScene.didEnterBackgroundNotification,
            UIWindow.didBecomeHiddenNotification
        ]
        observers = notifications.map { name in
            NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
                self?.cancelIconPlayback()
            }
        }
    }

    required init?(coder: NSCoder) { nil }

    deinit {
        iconHosts.forEach { $0.cancelPlayback() }
        observers.forEach { NotificationCenter.default.removeObserver($0) }
    }

    func configureIcons(titles: [String], ids: [String]) {
        guard titles.count == NativeTabIconHost.Artwork.allCases.count, titles.count == ids.count else { return }
        if iconHosts.isEmpty {
            iconHosts = NativeTabIconHost.Artwork.allCases.enumerated().map { index, artwork in
                let item = UITabBarItem(title: nil, image: UIImage(systemName: artwork.rawValue), tag: index)
                return NativeTabIconHost(item: item, artwork: artwork)
            }
            items = iconHosts.map(\.item)
        }
        for (index, icon) in iconHosts.enumerated() {
            icon.item.accessibilityLabel = titles[index]
            icon.item.accessibilityIdentifier = "runalytics.tab.\(ids[index])"
        }
    }

    func replayIcon(at index: Int) {
        cancelIconPlayback()
        guard iconHosts.indices.contains(index), !UIAccessibility.isReduceMotionEnabled,
              !isHidden, alpha > 0, let window = window, !window.isHidden,
              UIApplication.shared.applicationState == .active,
              window.windowScene.map({ $0.activationState == .foregroundActive }) ?? true else { return }
        layoutIfNeeded()
        iconHosts[index].replay()
    }

    func cancelIconPlayback() {
        iconHosts.forEach { $0.cancelPlayback() }
    }

    override var isHidden: Bool {
        didSet { if isHidden { cancelIconPlayback() } }
    }

    override var alpha: CGFloat {
        didSet { if alpha <= 0 { cancelIconPlayback() } }
    }

    override func willMove(toSuperview newSuperview: UIView?) {
        if newSuperview == nil { cancelIconPlayback() }
        super.willMove(toSuperview: newSuperview)
    }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil { cancelIconPlayback() }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        if bounds != iconLayout || safeAreaInsets != iconSafeArea {
            cancelIconPlayback()
            iconLayout = bounds
            iconSafeArea = safeAreaInsets
        }
    }
}
