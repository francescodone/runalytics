import XCTest
import UIKit
@testable import NativeChrome

final class NativeTabBarTests: XCTestCase {
    private let ids = ["training", "journal", "pace", "milestones"]

    private func makeBar() -> NativeTabBar {
        let bar = NativeTabBar(frame: CGRect(x: 0, y: 0, width: 390, height: 83))
        bar.configureIcons(titles: ["Training", "Activities", "Pace insights", "Milestones"], ids: ids)
        bar.selectedItem = bar.items?.first
        bar.layoutIfNeeded()
        return bar
    }

    func testRapidReplayIgnoresOldFramesAndRestoresNativeArtwork() {
        let bar = makeBar()
        for icon in bar.iconHosts {
            let original = icon.item.image
            let originalSize = bar.sizeThatFits(CGSize(width: 390, height: 844))
            icon.replay()
            let interrupted = icon.displayLink!
            icon.advance(to: 0)
            let restingFrame = icon.item.image!
            icon.advance(to: 0.15)
            XCTAssertNotEqual(icon.item.image?.pngData(), restingFrame.pngData())
            XCTAssertEqual(icon.item.image?.size, restingFrame.size)
            XCTAssertEqual(icon.item.image?.renderingMode, .alwaysTemplate)
            XCTAssertEqual(bar.sizeThatFits(CGSize(width: 390, height: 844)), originalSize)
            icon.replay()
            let replacement = icon.displayLink
            icon.tick(interrupted)
            XCTAssertTrue(icon.displayLink === replacement)
            XCTAssertTrue(icon.isPlaying)
            icon.advance(to: 0.56)
            XCTAssertFalse(icon.isPlaying)
            XCTAssertTrue(icon.item.image === original)
            XCTAssertTrue(icon.item.selectedImage === original)
        }
    }

    func testRunnerTakesAlternatingBentKneeStepsWithOpposingArms() {
        let poses = [0.0, 0.25, 0.5, 0.75].map { NativeRunnerGlyph.pose(at: $0) }
        XCTAssertGreaterThan(poses[0].frontLeg.tip.x, poses[0].backLeg.tip.x + 3)
        XCTAssertGreaterThan(poses[2].backLeg.tip.x, poses[2].frontLeg.tip.x + 3)
        XCTAssertGreaterThan(poses[1].frontLeg.tip.y, poses[1].backLeg.tip.y + 2)
        XCTAssertGreaterThan(poses[3].backLeg.tip.y, poses[3].frontLeg.tip.y + 2)
        XCTAssertGreaterThan(poses[0].frontLeg.joint.x, poses[2].frontLeg.joint.x + 5)
        XCTAssertGreaterThan(poses[2].frontArm.joint.x, poses[0].frontArm.joint.x + 4)
        let returning = poses[3].frontLeg
        let upper = CGPoint(x: returning.joint.x - returning.root.x, y: returning.joint.y - returning.root.y)
        let lower = CGPoint(x: returning.tip.x - returning.joint.x, y: returning.tip.y - returning.joint.y)
        XCTAssertLessThan((upper.x * lower.x + upper.y * lower.y) / (hypot(upper.x, upper.y) * hypot(lower.x, lower.y)), 0.5)
        for frame in 0...32 {
            let pose = NativeRunnerGlyph.pose(at: Double(frame) / 32)
            for limb in [pose.frontLeg, pose.backLeg, pose.frontArm, pose.backArm] {
                let root = limb.root
                let length = hypot(limb.joint.x - root.x, limb.joint.y - root.y)
                XCTAssertEqual(length, root == pose.hip ? 4.5 : hypot(2.5, 2.7), accuracy: 0.001)
                for point in [root, limb.joint, limb.tip] {
                    XCTAssertGreaterThanOrEqual(point.x, 1)
                    XCTAssertLessThanOrEqual(point.x, 23)
                    XCTAssertGreaterThanOrEqual(point.y, 1)
                    XCTAssertLessThanOrEqual(point.y, 23)
                }
            }
        }
    }

    func testRunnerFramesKeepTheirRestingArtworkMetricsAndTemplateTint() {
        let bar = makeBar()
        let icon = bar.iconHosts[1]
        let original = icon.item.image!
        icon.replay()
        icon.advance(to: 0)
        XCTAssertEqual(icon.item.image?.pngData(), original.pngData())
        var frames = Set<Data>()
        var rendered = [original]
        for phase in [0.125, 0.25, 0.375, 0.625, 0.75, 0.875] {
            icon.advance(to: phase * NativeTabIconHost.replayDuration)
            let image = icon.item.image!
            frames.insert(image.pngData()!)
            rendered.append(image)
            XCTAssertEqual(image.size, original.size)
            XCTAssertEqual(image.alignmentRectInsets, original.alignmentRectInsets)
            XCTAssertEqual(image.baselineOffsetFromBottom, original.baselineOffsetFromBottom)
            XCTAssertEqual(image.renderingMode, .alwaysTemplate)
            XCTAssertEqual(icon.item.selectedImage?.pngData(), image.pngData())
        }
        XCTAssertGreaterThan(frames.count, 2)
        icon.cancelPlayback()
        XCTAssertTrue(icon.item.image === original)
        let sheet = UIGraphicsImageRenderer(size: CGSize(width: 560, height: 240)).image { context in
            for row in 0...1 {
                (row == 0 ? UIColor.white : UIColor.black).setFill()
                context.fill(CGRect(x: 0, y: CGFloat(row * 120), width: 560, height: 120))
                for (index, image) in rendered.enumerated() {
                    let tinted = image.withTintColor(row == 0 ? .black : .white, renderingMode: .alwaysOriginal)
                    tinted.draw(in: CGRect(x: CGFloat(index * 80 + 8), y: CGFloat(row * 120 + 8), width: 64, height: 64))
                    tinted.draw(at: CGPoint(x: CGFloat(index * 80 + 28), y: CGFloat(row * 120 + 84)))
                }
            }
        }
        let attachment = XCTAttachment(image: sheet)
        attachment.name = "Native runner poses at enlarged and actual tab size"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    func testReplayFinishesOnItsOwnAndDoesNotRetainItsHost() {
        let item = UITabBarItem(title: nil, image: UIImage(systemName: "figure.run"), tag: 1)
        let host = NativeTabIconHost(item: item, artwork: .runner)
        let original = item.image
        host.replay()
        let finished = expectation(description: "Bounded native replay completes")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.75) {
            XCTAssertFalse(host.isPlaying)
            XCTAssertTrue(item.image === original)
            finished.fulfill()
        }
        wait(for: [finished], timeout: 2)
        weak var released: NativeTabIconHost?
        let transientItem = UITabBarItem(title: nil, image: UIImage(systemName: "figure.run"), tag: 1)
        var transientOriginal: UIImage?
        autoreleasepool {
            let transient = NativeTabIconHost(item: transientItem, artwork: .runner)
            transientOriginal = transientItem.image
            released = transient
            transient.replay()
            XCTAssertTrue(transient.isPlaying)
            transient.advance(to: 0.15)
        }
        XCTAssertNil(released)
        XCTAssertTrue(transientItem.image === transientOriginal)
        XCTAssertTrue(item.image === original)
    }

    func testLifecycleInterruptionsConsumeReplayWithoutResumingIt() {
        let bar = makeBar()
        let original = bar.iconHosts[1].item.image
        for notification in [UIAccessibility.reduceMotionStatusDidChangeNotification,
                             UIApplication.willResignActiveNotification,
                             UIApplication.didEnterBackgroundNotification,
                             UIScene.willDeactivateNotification,
                             UIScene.didEnterBackgroundNotification,
                             UIWindow.didBecomeHiddenNotification] {
            bar.iconHosts[1].replay()
            NotificationCenter.default.post(name: notification, object: nil)
            XCTAssertFalse(bar.iconHosts[1].isPlaying, notification.rawValue)
            XCTAssertTrue(bar.iconHosts[1].item.image === original)
            XCTAssertNil(bar.iconHosts[1].displayLink)
        }
        NotificationCenter.default.post(name: UIApplication.didBecomeActiveNotification, object: nil)
        XCTAssertTrue(bar.iconHosts.allSatisfy { !$0.isPlaying })
        bar.replayIcon(at: 0)
        XCTAssertFalse(bar.iconHosts[0].isPlaying, "Disconnected bars cannot play")
    }

    func testHideRemovalAndResizeRestoreArtwork() {
        let host = UIView(frame: CGRect(x: 0, y: 0, width: 390, height: 844))
        let bar = makeBar()
        host.addSubview(bar)
        bar.iconHosts[0].replay()
        bar.isHidden = true
        XCTAssertFalse(bar.iconHosts[0].isPlaying)
        bar.isHidden = false
        XCTAssertNil(bar.iconHosts[0].displayLink)
        bar.iconHosts[2].replay()
        bar.alpha = 0
        XCTAssertFalse(bar.iconHosts[2].isPlaying)
        bar.alpha = 1
        bar.iconHosts[2].replay()
        bar.frame.size.width = 320
        bar.layoutIfNeeded()
        XCTAssertFalse(bar.iconHosts[2].isPlaying)
        bar.iconHosts[3].replay()
        bar.removeFromSuperview()
        XCTAssertFalse(bar.iconHosts[3].isPlaying)
        XCTAssertTrue(bar.iconHosts.allSatisfy { !$0.isPlaying })
    }

    func testSnapshotUpdatesRetainControlsSelectionAndAccessibilityDuringReplay() {
        let bar = makeBar()
        let items = bar.items!
        let icons = bar.iconHosts
        let selected = bar.selectedItem
        icons[1].replay()
        bar.configureIcons(titles: ["Allenamento", "Attività", "Ritmo", "Traguardi"], ids: ids)
        XCTAssertTrue(bar.selectedItem === selected)
        XCTAssertTrue(zip(items, bar.items!).allSatisfy { $0 === $1 })
        XCTAssertTrue(zip(icons, bar.iconHosts).allSatisfy { $0 === $1 })
        XCTAssertEqual(items[1].accessibilityLabel, "Attività")
        XCTAssertEqual(items[1].accessibilityIdentifier, "runalytics.tab.journal")
        XCTAssertTrue(icons[1].isPlaying)
        bar.cancelIconPlayback()
    }
}
