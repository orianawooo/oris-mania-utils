export const VISUAL_SHAPES = new Set(['rounded', 'pill', 'circle', 'square', 'diamond', 'hex']);

export function normalizeScale(value, fallback = 1) {
    const n = Number(value);
    return Number.isFinite(n) && n > 0 ? n : fallback;
}

export function normalizeRotation(value, fallback = 0) {
    const n = Number(value);
    return Number.isFinite(n) ? n : fallback;
}

export function normalizeShape(value, fallback = 'rounded') {
    return VISUAL_SHAPES.has(value) ? value : fallback;
}

export function toRadians(degrees) {
    return (degrees * Math.PI) / 180;
}

export function shapeRadius(shape, width, height, baseRadius = 12) {
    if (shape === 'pill' || shape === 'circle') {
        return Math.min(width, height) / 2;
    }
    if (shape === 'square' || shape === 'diamond' || shape === 'hex') {
        return 0;
    }
    return Math.min(baseRadius, width / 2, height / 2);
}

function traceRoundedRectPath(ctx, x, y, width, height, radius) {
    ctx.beginPath();
    ctx.moveTo(x + radius, y);
    ctx.lineTo(x + width - radius, y);
    ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
    ctx.lineTo(x + width, y + height - radius);
    ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
    ctx.lineTo(x + radius, y + height);
    ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
    ctx.lineTo(x, y + radius);
    ctx.quadraticCurveTo(x, y, x + radius, y);
    ctx.closePath();
}

export function traceShapePath(ctx, x, y, width, height, shape = 'rounded', baseRadius = 12) {
    const safeShape = normalizeShape(shape, 'rounded');
    if (safeShape === 'diamond') {
        ctx.beginPath();
        ctx.moveTo(x + width / 2, y);
        ctx.lineTo(x + width, y + height / 2);
        ctx.lineTo(x + width / 2, y + height);
        ctx.lineTo(x, y + height / 2);
        ctx.closePath();
        return;
    }

    if (safeShape === 'hex') {
        const inset = width * 0.2;
        ctx.beginPath();
        ctx.moveTo(x + inset, y);
        ctx.lineTo(x + width - inset, y);
        ctx.lineTo(x + width, y + height / 2);
        ctx.lineTo(x + width - inset, y + height);
        ctx.lineTo(x + inset, y + height);
        ctx.lineTo(x, y + height / 2);
        ctx.closePath();
        return;
    }

    traceRoundedRectPath(ctx, x, y, width, height, shapeRadius(safeShape, width, height, baseRadius));
}

export function withElementTransform(ctx, x, y, width, height, scale = 1, rotation = 0, draw) {
    const cx = x + width / 2;
    const cy = y + height / 2;
    ctx.save();
    ctx.translate(cx, cy);
    ctx.rotate(toRadians(rotation));
    ctx.scale(scale, scale);
    ctx.translate(-cx, -cy);
    draw();
    ctx.restore();
}

export function getTransformedBounds(x, y, width, height, scale = 1, rotation = 0) {
    const scaledW = width * scale;
    const scaledH = height * scale;
    const radians = toRadians(rotation);
    const cos = Math.cos(radians);
    const sin = Math.sin(radians);
    const boundsW = Math.abs(scaledW * cos) + Math.abs(scaledH * sin);
    const boundsH = Math.abs(scaledW * sin) + Math.abs(scaledH * cos);
    const cx = x + width / 2;
    const cy = y + height / 2;

    return {
        x: cx - boundsW / 2,
        y: cy - boundsH / 2,
        w: boundsW,
        h: boundsH,
    };
}

export function resolveTrailAnchor({
    lockTrails = true,
    transformedCenterX = 0,
    transformedTopY = 0,
    keyOffsetX = 0,
    keyOffsetY = 0,
    trailOffsetX = 0,
    trailOffsetY = 0,
} = {}) {
    if (lockTrails) {
        return {
            anchorX: transformedCenterX,
            anchorY: transformedTopY,
        };
    }

    return {
        anchorX: transformedCenterX - keyOffsetX + trailOffsetX,
        anchorY: transformedTopY - keyOffsetY + trailOffsetY,
    };
}

export function getTrailBounds({
    anchorX = 0,
    anchorY = 0,
    width = 0,
    height = 0,
    scale = 1,
    rotation = 0,
} = {}) {
    const scaledWidth = Math.max(4, width * normalizeScale(scale, 1));
    const scaledHeight = Math.max(4, height * normalizeScale(scale, 1));

    return getTransformedBounds(
        anchorX - scaledWidth / 2,
        anchorY - scaledHeight,
        scaledWidth,
        scaledHeight,
        1,
        rotation,
    );
}
